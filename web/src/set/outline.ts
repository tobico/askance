//! The shape of a Set's page as the table of contents has it: which sections it
//! is made of, the ids they are reached by, and where the highlight belongs
//! among them.
//!
//! All of it worked out from the Set rather than from the page. The nav lists
//! what the Set has, so a section the Set is without is a section the nav does
//! not offer — and the scroll-spy watches the same list, so the nav and the spy
//! cannot disagree about what the page is made of. A disagreement there is a
//! highlight sitting on a line that is not where the reader is.
//!
//! Kept apart from the drawing of it for the same reason: this is the one
//! description both shapes of the nav and the floating header are built from.

import type { SetView } from "../api/types";

/// The id a Question is reached by: its label, lowercased — `Q3` becomes `q3`,
/// which is also what a human writing the link by hand would type.
///
/// A label is the agent's own string, and an id cannot hold everything a string
/// can, so anything an id will not take becomes a hyphen; a label made of
/// nothing else falls back to the Question's position. Labels are distinct
/// across a Set and in practice they are `Q1`, `Q2`, …, so the fallback is for
/// the pathological Set rather than the ordinary one.
///
/// Sub-questions get none: one scrolls into view with its parent.
export function anchor(label: string, position: number): string {
  const id = [...label.trim().toLowerCase()]
    .map((letter) => (/[a-z0-9\-_]/.test(letter) ? letter : "-"))
    .join("")
    .replace(/^-+/, "")
    .replace(/-+$/, "");

  return id === "" ? `q${position}` : id;
}

/// How much of a file's path the table of contents has room for, in characters.
///
/// A count rather than a measurement: the paths are monospaced, so characters
/// are what the column is made of — and a cut made here can land on a directory
/// boundary, where one made by the column would land wherever the pixels ran
/// out. The stylesheet still clips what overruns, for the narrow end of the
/// range the nav is drawn across; this is what keeps the ordinary path legible.
const PATH_ROOM = 24;

/// A file's path as the table of contents shows it: whole when it fits, and
/// otherwise cut from the *left* — `crates/app/src/set_view.rs` becomes
/// `…/app/src/set_view.rs`.
///
/// From the left because the end of a path is the part being looked for: a
/// column of `crates/app/src/…` names nothing. The cut lands on a directory
/// boundary so what is left is still a path, and only cuts into the filename
/// itself when the filename alone is longer than the line — at which point
/// something has to give, and the extension is worth more than the stem.
export function shortened(path: string): string {
  const letters = [...path];
  if (letters.length <= PATH_ROOM) {
    return path;
  }

  // What is left once the leading `…/` has taken its two.
  const room = PATH_ROOM - 2;

  // The first boundary whose tail fits is the one that keeps the most of the
  // path.
  for (const [at, letter] of letters.entries()) {
    if (letter !== "/") {
      continue;
    }
    const tail = letters.slice(at + 1);
    if (tail.length <= room) {
      return `…/${tail.join("")}`;
    }
  }

  return `…${letters.slice(letters.length - (room + 1)).join("")}`;
}

/// One line of the table of contents under a section heading: a file of the
/// Diff, or a Question.
export type Entry = {
  /// The id it jumps to, without the `#`.
  anchor: string;

  /// The name a Question answers to, kept apart from the words beside it so it
  /// can be styled as the fixed thing it is. A file has none — its path is the
  /// whole of what it is called.
  label: string | null;

  /// What the line reads as, already cut down to what will fit if it is a path.
  /// A Question's words are left whole and cut by the column.
  text: string;

  /// The whole of it, for the browser's own tooltip: the nav is narrow, and
  /// this is where the truncated line can be read out in full.
  whole: string;
};

/// One section of the page in the table of contents: the heading it jumps to,
/// and whatever the section is made of.
export type Section = {
  anchor: string;
  name: string;
  entries: Entry[];
};

/// The page's sections top to bottom, each with its own parts under it.
export function outline(set: SetView): Section[] {
  const sections: Section[] = [];

  if (set.preface_html !== null) {
    sections.push({ anchor: "preface", name: "Preface", entries: [] });
  }

  if (set.diff !== null) {
    sections.push({
      anchor: "diff",
      name: "Diff",
      entries: set.diff.paths.map((path, index) => ({
        // The renderer counts the folds from one, and these are the same folds
        // in the same order.
        anchor: `diff-${index + 1}`,
        label: null,
        text: shortened(path),
        whole: path,
      })),
    });
  }

  // Unconditional, like the heading it points at: every Set has Questions.
  sections.push({
    anchor: "questions",
    name: "Questions",
    entries: set.questions.map((question, index) => ({
      anchor: anchor(question.ask.name, index + 1),
      label: question.ask.name,
      // The words rather than the rendered text: this is a line in a column,
      // and the markup the Question is drawn with has no place in it. See
      // `QuestionView.nav_text`.
      text: question.nav_text,
      whole: `${question.ask.name} ${question.nav_text}`,
    })),
  });

  // Sub-questions are not listed: one scrolls into view with its parent, and a
  // nav that listed them would be the page again rather than a way around it.
  return sections;
}

/// The class that sets one line's words: a Question's label and prose in the
/// page's own face, a file's path in the Diff's, so that a path in the nav and
/// the same path over its fold read as the same name.
///
/// Taken from whether there is a label because that is what tells the two
/// apart: a file's path is the whole of what it is called.
export function face(label: string | null): string {
  return label === null ? "contents-path" : "contents-question";
}

/// One anchored part of the page as the nav has it: the id the scroll-spy
/// watches, and the name to put on it.
///
/// The name travels with the id because three things read it out — the
/// sidebar's own line, the bar, and the floating header, each of which says
/// nothing but the name of the line the highlight is on. Kept as one list
/// rather than as a list of ids beside a list of names, so none of them can
/// name a different part of the page than the one the spy is pointing at.
export type Watched = {
  /// The id it jumps to, without the `#`.
  anchor: string;

  /// The name a Question answers to, kept apart from the words beside it. A
  /// section and a file have none.
  label: string | null;

  /// What the line reads as: a section's name, a path already cut to what will
  /// fit, or a Question's words.
  text: string;

  /// The class that sets those words — see [`face`].
  kind: string;

  /// The section this part of the page belongs to, by the id and the name of
  /// it. A section heading belongs to itself, so for one of those these are its
  /// own — which is how [`inside`] tells the two apart without a flag.
  ///
  /// Carried rather than worked out by walking back up the list: the header
  /// names the section along with the part inside it, and the wrap switch has
  /// to know whether the reader is anywhere in the Diff at all.
  section: string;
  sectionName: string;
};

/// The section this sits inside, or `null` when it *is* the section — which is
/// exactly when its own id is the section's.
export function inside(watched: Watched): string | null {
  return watched.anchor === watched.section ? null : watched.sectionName;
}

/// Every anchored part of the page, in page order — the ids the scroll-spy
/// watches, and what [`lit`]'s answer counts along.
///
/// Each section's own heading and then whatever is under it, which is the order
/// the page has them in. The two levels do not fight over the highlight because
/// [`lit`] takes the *last* part to have begun: a file always begins after the
/// Diff heading it is under, so the file wins for as long as the reader is in
/// it, and the heading only holds the highlight in the gap above the first
/// file.
export function spied(sections: Section[]): Watched[] {
  return sections.flatMap((section) => [
    {
      anchor: section.anchor,
      label: null,
      text: section.name,
      kind: "contents-section",
      section: section.anchor,
      sectionName: section.name,
    },
    ...section.entries.map((entry) => ({
      anchor: entry.anchor,
      label: entry.label,
      text: entry.text,
      kind: face(entry.label),
      section: section.anchor,
      sectionName: section.name,
    })),
  ]);
}

/// Where this id sits among the parts the spy watches, and `null` when it is
/// none of them.
export function spot(watched: Watched[], anchor: string): number | null {
  const at = watched.findIndex((watched) => watched.anchor === anchor);
  return at === -1 ? null : at;
}

/// What one line of the nav answers for, as places along the watched list.
export type Stands = {
  /// The part of the page it names. The line is *the* highlight while the
  /// reader is exactly here.
  at: number;

  /// The last place under it: its own, unless it is a section with files or
  /// Questions beneath it, and then the last of those. While the reader is
  /// anywhere in between, the line says so quietly and leaves the highlight to
  /// whichever of its entries they are actually in — so a file lit up reads as
  /// being within the Diff without the two competing for the same mark.
  through: number;
};

/// What the line naming this section answers for: its own heading, reaching
/// down through the last of whatever is under it.
export function standsFor(watched: Watched[], section: Section): Stands | null {
  const at = spot(watched, section.anchor);
  if (at === null) {
    return null;
  }

  const last = section.entries.at(-1);
  const through = last === undefined ? null : spot(watched, last.anchor);

  return { at, through: through ?? at };
}

/// One part of the page with nothing under it: a file, or a Question.
export function just(at: number): Stands {
  return { at, through: at };
}

/// How the highlight touches one line of the nav: the reader is at this very
/// part of the page, or somewhere inside it — a section drawn over the file or
/// Question they are actually in, said quietly so that it never competes with
/// the line that says where they are.
export type Mark = "at" | "within";

/// The class that draws it.
export function markClass(mark: Mark): string {
  return mark === "at" ? "contents-here" : "contents-within";
}

/// Which mark a line standing for these places carries while the reader is at
/// `here`, and none when they are elsewhere in the page.
///
/// The two are exclusive by construction rather than by being drawn carefully:
/// there is one mark per line and the loud one wins, so a section at its own
/// heading is never quietly marked as containing itself.
export function mark(stands: Stands | null, here: number): Mark | null {
  if (stands === null) {
    return null;
  }

  if (stands.at === here) {
    return "at";
  }

  return stands.at <= here && here <= stands.through ? "within" : null;
}

/// Which of the spied parts of the page the highlight belongs on, given which
/// of them have started above the reading line.
///
/// The last one to have started. Their tops run down the page in this order, so
/// the last one to have passed the line is the one whose text is under it — and
/// asking it this way rather than as "the one across the line" means the answer
/// never depends on where the reader came from, only on where the page is now.
///
/// With none begun, the line is still above the first of them, which is the top
/// of the page: that counts as being in the first section, so the first line is
/// the lit one. It is also the answer before the spy has run at all, which is
/// the page as it is first drawn.
export function lit(started: boolean[]): number {
  const last = started.lastIndexOf(true);
  return last === -1 ? 0 : last;
}
