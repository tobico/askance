//! The set view: one Question Set laid out to be read — its Preface, then every
//! Question and Sub-question in the order the agent asked them.
//!
//! A Set that has settled gets the same page read rather than filled in: its own
//! material above, and under it what was decided — the Option chosen beside the
//! one the agent recommended, whatever was written, and the questions that went
//! back open. A Set is answered once, so there is nothing here to press. Which of
//! the two the page draws is decided from the Set as the server loads it, so an
//! answered Set never flashes a form.
//!
//! A Set that was archived reads like an answered one — permanently, and with
//! nothing to press — except that there is no Response to show, because there
//! never was one.

import { A, useParams } from "@solidjs/router";
import { useQuery } from "@tanstack/solid-query";
import type { JSX } from "solid-js";
import { For, Match, Show, Switch, onCleanup, onMount } from "solid-js";

import { RefusedError, loadSet } from "../api/client";
import type {
  Answer,
  AskView,
  OptionView,
  QuestionView,
  Response,
  SetView,
} from "../api/types";
import { drawDiagrams } from "./diagrams";
import { settledWhen } from "./when";

/// One Question Set, as the URL names it.
export function SetPage(): JSX.Element {
  const params = useParams<{ id: string }>();

  const set = useQuery(() => ({
    queryKey: ["set", params.id],
    queryFn: () => loadSet(params.id),
  }));

  return (
    <Switch>
      {/* Pending rather than fetching: the fallback belongs to the first load
          alone. */}
      <Match when={set.isPending}>
        <p class="empty">Loading…</p>
      </Match>
      <Match when={set.isError && absent(set.error)}>
        <p class="empty">No such Set.</p>
      </Match>
      <Match when={set.isError}>
        <p class="error">Could not read the Set: {set.error?.message}</p>
      </Match>
      <Match when={set.data}>{(set) => <Sheet set={set()} />}</Match>
    </Switch>
  );
}

/// Whether there is simply no such Set — which the server says with a 404, and
/// which is a page to draw rather than a failure to report. An id that is not a
/// number gets the same answer, because it cannot name a Set either.
function absent(error: Error | null): boolean {
  return error instanceof RefusedError && error.status === 404;
}

/// One Set, top to bottom: how it stands and its own material — what the agent
/// asked about — and then the record of what became of it.
///
/// The material above is the same however it stands: a settled Set is read for
/// what was decided *and* for what the decision was about.
function Sheet(props: { set: SetView }): JSX.Element {
  // The renderer, named by a Set that has a Diagram on it to draw and by no
  // other: mermaid is megabytes, so a Set without one loads none of them. What a
  // page that does ask for it gets is the source blocks the markdown renderer
  // already wrote, drawn over — and if the bundle never arrives, the blocks
  // themselves, which is a readable page and not a broken one.
  //
  // Once, on mount, rather than from an effect over the Set: whether this page
  // carries a renderer is a fact about the Set it is drawing, and drawing the
  // same one again when the Set is fetched afresh would find every block already
  // replaced.
  onMount(() => {
    if (!props.set.diagrams) {
      return;
    }

    onCleanup(drawDiagrams());
  });

  const standing = () => props.set.standing;

  /// Whether this Set has settled, one way or the other. It is what decides
  /// whether the page reads as a record.
  const decided = () => !("Waiting" in standing());

  /// The Response behind the record, and `null` when there is none: a Set closed
  /// unanswered is a record with nothing decided in it.
  const response = (): Response | null => {
    const how = standing();
    return "Answered" in how ? how.Answered.response : null;
  };

  /// When the Set settled, said in words, and what to call the settling. Beside
  /// the provenance rather than down with the Answers: on a settled Set, when it
  /// was settled is part of knowing what one is reading — and for one that was
  /// closed unanswered it is most of what there is to know.
  const when = () => {
    const how = standing();

    if ("Answered" in how) {
      return {
        // A class of its own for each: the two lines sit in the same place and
        // are styled together, but nothing about an archived Set was answered.
        mark: "answered-at",
        said: `Answered ${settledWhen(how.Answered.submitted_at)}`,
      };
    }

    if ("ArchivedUnanswered" in how) {
      return {
        mark: "archived-at",
        said: `Archived unanswered ${settledWhen(how.ArchivedUnanswered)}`,
      };
    }

    return null;
  };

  // Back to the list this Set is on: a settled one is off the pending list for
  // good and lives in the Archive, so that is where reading it leads back to.
  const back = () => (decided() ? "/archive" : "/");
  const out = () => (decided() ? "← Archive" : "← Pending");

  return (
    <>
      <A href={back()} class="back">
        {out()}
      </A>
      <h1>{props.set.title}</h1>
      {/* A Set sent from outside a repo has neither, and an empty line of
          provenance is worse than none. */}
      <Show when={props.set.project !== null || props.set.branch !== null}>
        <p class="meta">
          <Show when={props.set.project}>
            {(project) => <span class="project">{project()}</span>}
          </Show>
          <Show when={props.set.branch}>
            {(branch) => <span class="branch">{branch()}</span>}
          </Show>
        </p>
      </Show>
      <Show when={when()}>
        {(when) => <p class={when().mark}>{when().said}</p>}
      </Show>
      {/* Named and anchored like the Questions below it: the heading is what a
          jump from the table of contents lands on, and the id is what it jumps
          to.

          The body is marked as rendered markdown, so the agent's headings,
          tables and code get the same rules there as they get inside a Question
          — the section around it is all that is the Preface's own. */}
      <Show when={props.set.preface_html}>
        {(html) => (
          <section class="preface" id="preface">
            <h2 class="section-heading">Preface</h2>
            <div class="preface-body markdown" innerHTML={html()} />
          </section>
        )}
      </Show>
      {/* The Diff goes between the Preface and the Questions — the Preface says
          what the agent is asking about, and the Diff is the evidence for it. It
          is its own task, and until then a Set that has one is read without it. */}
      <Questions
        questions={props.set.questions}
        decided={decided()}
        response={response()}
      />
    </>
  );
}

/// The questions, and what became of them.
///
/// The heading is drawn unconditionally, unlike the Preface's: the Questions are
/// the one section every Set has. Its id sits on the heading rather than on the
/// list, so a jump lands on the name of the thing rather than just above its
/// first row.
function Questions(props: {
  questions: QuestionView[];
  decided: boolean;
  response: Response | null;
}): JSX.Element {
  /// A Set that settled with no Response behind it, which is the one standing
  /// that was never answered by anybody.
  const orphaned = () => props.decided && props.response === null;

  /// What to say at the head of a Response that resolved nothing.
  const nothing = () => {
    const response = props.response;
    return response === null ? null : nothingAnswered(response);
  };

  /// Shown only when there is one, exactly as the submit only ever sends one that
  /// has something in it.
  const comment = () => {
    const said = props.response?.comment?.trim();
    return said === undefined || said === "" ? null : said;
  };

  return (
    <>
      {/* Above the heading: what a Response resolved — or did not — is said at
          the head of the page, about the Set as a whole, not under the
          Questions. */}
      <Show when={orphaned()}>
        <p class="counter-question">
          This Set was archived unanswered: nobody answered these questions, and
          no Response was ever sent. The agent was told the Set had been
          archived.
        </p>
      </Show>
      <Show when={nothing()}>
        {(said) => <p class="counter-question">{said()}</p>}
      </Show>
      <h2 class="section-heading" id="questions">
        Questions
      </h2>
      <ol class={props.decided ? "questions decided" : "questions"}>
        <For each={props.questions}>
          {(question, index) => (
            <Question
              question={question}
              position={index() + 1}
              decided={props.decided}
              response={props.response}
            />
          )}
        </For>
      </ol>
      <Show when={comment()}>
        {(comment) => (
          <section class="set-comment decided">
            <h2>On the Set as a whole</h2>
            <p class="comment">{comment()}</p>
          </section>
        )}
      </Show>
    </>
  );
}

/// One Question, with its Sub-questions nested one level under it.
function Question(props: {
  question: QuestionView;
  position: number;
  decided: boolean;
  response: Response | null;
}): JSX.Element {
  return (
    <li
      class="question"
      id={anchor(props.question.ask.name, props.position)}
    >
      <Ask
        ask={props.question.ask}
        decided={props.decided}
        response={props.response}
      />
      {/* Sub-questions get no anchor of their own: one scrolls into view with
          its parent. */}
      <Show when={props.question.subquestions.length > 0}>
        <ol class="subquestions">
          <For each={props.question.subquestions}>
            {(subquestion) => (
              <li class="subquestion">
                <Ask
                  ask={subquestion}
                  decided={props.decided}
                  response={props.response}
                />
              </li>
            )}
          </For>
        </ol>
      </Show>
    </li>
  );
}

/// A Question or a Sub-question — both are read the same way: the name it answers
/// to, its text as the server rendered it, every Option it offered, and, once
/// there is a Response, what became of it.
///
/// Every Option is kept, not just the chosen one: what was turned down is half of
/// what a decision was.
///
/// Given the whole Response rather than this question's entry, because the absence
/// of a Response is itself something to draw: with none at all the Set was
/// archived unanswered, and there was nobody to tell that these questions were
/// still open.
function Ask(props: {
  ask: AskView;
  decided: boolean;
  response: Response | null;
}): JSX.Element {
  const answer = () => {
    const response = props.response;
    return response === null ? undefined : answerTo(response, props.ask.name);
  };

  const selected = () => answer()?.selected ?? null;

  const said = () => {
    const words = answer()?.free_text?.trim();
    return words === undefined || words === "" ? null : words;
  };

  // No Option and no words is the Unanswered marker, whether or not the flag is
  // set: either way nothing was answered here. Only a Response can leave a
  // question open, though — an archived Set says so once, at the head of the
  // page, rather than claiming the agent was told anything.
  const open = () =>
    props.response !== null && selected() === null && said() === null;

  // The form's own wording, minus the name of the Question it prefixes there: a
  // field in a column of five needs telling apart from the other four, and this
  // sits inside the one Question it belongs to with nothing to be confused with.
  const prompt = () =>
    props.ask.options.length === 0 ? "Your answer" : "Your thoughts";

  return (
    <div class={props.decided ? "ask decided" : "ask"}>
      {/* The label a Response answers by, then the text it labels — kept a child
          of its own rather than being swallowed by the rendered markup beside
          it, however blocky the agent's markdown under it is. */}
      <div class="text">
        <span class="label">{props.ask.name}</span>
        <div class="markdown" innerHTML={props.ask.text_html} />
      </div>
      <Show when={props.ask.options.length > 0}>
        <ul class="options">
          <For each={props.ask.options}>
            {(option) => <Offered option={option} selected={selected()} />}
          </For>
        </ul>
      </Show>
      <Show when={said()}>
        {(said) => (
          <p class="answer-text">
            <span class="prompt">{prompt()}</span>
            {said()}
          </p>
        )}
      </Show>
      <Show when={open()}>
        <p class="unanswered">
          Unanswered — the agent was told this one is still open.
        </p>
      </Show>
    </div>
  );
}

/// One Option as it was offered: numbered and worded as the agent wrote it,
/// marked if they recommended it, and marked apart from that if this is the one
/// the human chose.
///
/// The two marks are deliberately different things to read: the ★ is what was
/// suggested, and the outline is what was decided, which on any given question may
/// well not be the same Option.
///
/// "chosen" is still written, and the stylesheet takes it out of the layout rather
/// than out of the page — the outline says which one to a reader looking at it and
/// nothing at all to one who is not, and an archive that cannot say what was
/// decided is not much of an archive. See `.ask.decided .chose`.
function Offered(props: {
  option: OptionView;
  selected: number | null;
}): JSX.Element {
  const chosen = () => props.selected === props.option.n;

  const marks = () =>
    ["option", chosen() && "chosen", props.option.recommended && "recommended"]
      .filter(Boolean)
      .join(" ");

  // The text is filled in wholesale, and it is inline markup all the way down —
  // the rendering flattened anything blockier on the way here, because an Option
  // is one line beside its number. It is marked as rendered markdown all the
  // same: what did survive, a code span above all, is drawn as it is everywhere
  // else.
  return (
    <li class={marks()}>
      <span class="n">{props.option.n}</span>
      <span
        class="option-text markdown"
        innerHTML={props.option.text_html}
      />
      <Show when={props.option.recommended}>
        <span class="star" title="the agent's Recommendation">
          ★
        </span>
      </Show>
      <Show when={chosen()}>
        <span class="chose">chosen</span>
      </Show>
    </li>
  );
}

/// The Response's entry for this question, if it has one.
///
/// A stored Response was validated against its Set, so every question has exactly
/// one entry. The lookup is still fallible because the page draws the Set rather
/// than the Response: a question with nothing to show reads as Unanswered, which
/// is true of one, rather than as a gap in the page.
function answerTo(response: Response, name: string): Answer | undefined {
  return response.answers.find((answer) => answer.label.trim() === name);
}

/// Whether an entry carries an Answer at all — an Option was selected or something
/// was written. One that carries neither is the Unanswered marker.
function isAnswer(answer: Answer): boolean {
  return (
    (answer.selected ?? null) !== null ||
    (answer.free_text ?? "").trim() !== ""
  );
}

/// What to say at the head of a Response that resolved nothing — and `null` when
/// it resolved something, which is the ordinary case.
///
/// Answering a Set by leaving every question open is allowed: with the set-level
/// comment it is a counter-question, the human's "not these questions", and it is
/// as much a Response as any other. It has to read as one rather than as a page
/// whose Answers failed to arrive, which is what a column of Unanswered with no
/// word about why would look like.
function nothingAnswered(response: Response): string | null {
  if (response.answers.some(isAnswer)) {
    return null;
  }

  const commented = (response.comment ?? "").trim() !== "";

  return commented
    ? "Nothing here was answered. The comment below is the whole Response — a " +
        "counter-question — and every question went back to the agent still open."
    : "Nothing here was answered, and nothing was said about the Set either: " +
        "every question went back to the agent still open.";
}

/// The id a Question is reached by: its label, lowercased — `Q3` becomes `q3`,
/// which is also what a human writing the link by hand would type.
///
/// A label is the agent's own string, and an id cannot hold everything a string
/// can, so anything an id will not take becomes a hyphen; a label made of nothing
/// else falls back to the Question's position. Labels are distinct across a Set
/// and in practice they are `Q1`, `Q2`, …, so the fallback is for the pathological
/// Set rather than the ordinary one.
function anchor(label: string, position: number): string {
  const id = [...label.trim().toLowerCase()]
    .map((letter) => (/[a-z0-9\-_]/.test(letter) ? letter : "-"))
    .join("")
    .replace(/^-+/, "")
    .replace(/-+$/, "");

  return id === "" ? `q${position}` : id;
}
