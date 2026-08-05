//! The answer sheet: what the human has put in it, the Response it adds up to,
//! and where it is kept between visits.
//!
//! None of this knows anything about the page it is filled in on. The Response
//! a sheet becomes is arithmetic — an entry per question, whitespace not
//! counting for anything — and a draft is that same arithmetic held still.

import type { Answer, Response } from "../api/types";
import { forget, read, write } from "../device";

/// One question's fields as the human left them, away from the signals holding
/// them — the shape [`drafted`] turns into a Response, and the shape a draft is
/// stored as.
export type Filled = {
  /// The name the question answers to: `Q7` for a Question, `Q7a` for a
  /// Sub-question.
  label: string;
  selected: number | null;
  free_text: string;
};

/// A half-finished answer sheet, as it sits in `localStorage` between visits:
/// every question's fields in the order the Set asked them, plus the set-level
/// comment.
///
/// The same per-question shape a submit snapshots, so a draft serializes as the
/// sheet stands rather than through a parallel one.
///
/// Deliberately per device and never sent to the server: a phone and a laptop
/// keep their own drafts of the same Set.
export type Draft = { filled: Filled[]; comment: string };

/// Whether the human has put anything into this question: an Option, words, or
/// both. Whitespace is not an answer here any more than it is at submit.
export function answered(field: Filled): boolean {
  return field.selected !== null || field.free_text.trim() !== "";
}

/// The Response a page full of fields adds up to.
///
/// Every question gets an entry, in the order it was asked: an Answer when the
/// human put something in, the Unanswered marker when they did not. Whitespace
/// is not an answer, and neither is a blank comment.
///
/// A field with nothing in it is left out of its entry rather than sent empty,
/// which is how the schema writes a Response everywhere else — what the CLI
/// parses and what the agent is handed as YAML.
export function drafted(filled: Filled[], comment: string): Response {
  const answers: Answer[] = filled.map((field) => {
    const words = field.free_text.trim();
    const answer: Answer = { label: field.label };

    if (field.selected !== null) {
      answer.selected = field.selected;
    }
    if (words !== "") {
      answer.free_text = words;
    }
    // Exclusive with an Answer, so it goes on exactly the entries that carry
    // nothing.
    if (!answered(field)) {
      answer.unanswered = true;
    }

    return answer;
  });

  const said = comment.trim();

  return said === "" ? { answers } : { answers, comment: said };
}

/// The questions the warning before submit names: those the Response leaves
/// open among the ones that offered Options. A free-text question left blank
/// still goes back marked Unanswered — it just draws no warning, because there
/// was no offered choice to overlook.
export function unanswered(
  response: Response,
  multipleChoice: string[],
): string[] {
  return response.answers
    .filter(
      (answer) => answer.unanswered && multipleChoice.includes(answer.label),
    )
    .map((answer) => answer.label);
}

/// What a click on Option `n` leaves the question holding, given what it held
/// already: nothing, when the click landed on the Option that was already
/// selected, and otherwise the Option clicked.
///
/// Clearing a question is the one thing a radio group cannot do on its own — the
/// browser has no gesture for un-picking one — and changing your mind about
/// approving a Recommendation you have thought better of is exactly the case
/// that wants it. A second click undoes the first, which is the only gesture
/// there was left to give it.
export function clicked(selected: number | null, n: number): number | null {
  return selected === n ? null : n;
}

/// Where one Set's draft lives. Keyed by the Set's id, so two Sets being
/// answered in turn keep independent drafts.
export function draftKey(id: number): string {
  return `askance.draft.${id}`;
}

/// Whether there is nothing in a draft to come back to. An untouched sheet is
/// not worth storing, and whitespace is no more an answer here than it is at
/// submit.
export function empty(draft: Draft): boolean {
  return draft.comment.trim() === "" && !draft.filled.some(answered);
}

/// The draft in `body`, if it is one this Set can be filled in from.
///
/// A body that will not parse, that is not the shape of a draft, or whose
/// questions are not this Set's asked in this order, is discarded whole rather
/// than applied in part: the Set as the agent sent it wins over a draft that no
/// longer describes it.
export function restorable(
  body: string | null,
  labels: string[],
): Draft | null {
  const draft = body === null ? null : parsed(body);
  if (draft === null) {
    return null;
  }

  const drafted = draft.filled.map((field) => field.label);
  const same =
    drafted.length === labels.length &&
    drafted.every((label, at) => label === labels[at]);

  return same ? draft : null;
}

/// The draft this Set left on this device, if there is one the sheet can be
/// filled in from. One that is no longer usable is dropped on the way past: it
/// will be no more use on the next visit than on this one.
export function storedDraft(id: number, labels: string[]): Draft | null {
  const key = draftKey(id);
  const body = read(key);
  if (body === null) {
    return null;
  }

  const draft = restorable(body, labels);
  if (draft === null) {
    forget(key);
  }

  return draft;
}

/// Write the draft out, replacing whatever was under the key — or drop it, when
/// there is nothing left in it: a draft of nothing would only ever restore as
/// nothing.
export function keepDraft(id: number, draft: Draft): void {
  const key = draftKey(id);

  if (empty(draft)) {
    forget(key);
  } else {
    write(key, JSON.stringify(draft));
  }
}

/// Drop this Set's draft, for a Set that will never take a Response from here.
export function clearDraft(id: number): void {
  forget(draftKey(id));
}

/// A draft out of its stored body, checked field by field.
///
/// Hand-checked because nothing on this side of the wire does it for us, and a
/// body under this key is whatever some older build of the viewer left there.
function parsed(body: string): Draft | null {
  let payload: unknown;
  try {
    payload = JSON.parse(body);
  } catch {
    return null;
  }

  if (typeof payload !== "object" || payload === null) {
    return null;
  }

  const { filled, comment } = payload as Partial<Draft>;
  if (!Array.isArray(filled) || typeof comment !== "string") {
    return null;
  }

  const fields: Filled[] = [];
  for (const field of filled as unknown[]) {
    if (typeof field !== "object" || field === null) {
      return null;
    }

    const { label, selected, free_text } = field as Partial<Filled>;
    if (typeof label !== "string" || typeof free_text !== "string") {
      return null;
    }
    if (selected !== null && typeof selected !== "number") {
      return null;
    }

    fields.push({ label, selected, free_text });
  }

  return { filled: fields, comment };
}
