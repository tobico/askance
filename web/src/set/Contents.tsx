//! The table of contents, and the reader's place in the page it describes.
//!
//! One nav for both of the shapes it takes: the sidebar in the wide margin, and
//! the bar with the list under it below that width. The same entries, the same
//! scroll-spy, and the same list of links either way — which of the two the
//! reader gets is the stylesheet's answer to how wide their window is, and
//! there is no second copy to fall out of step with the first.
//!
//! The floating header is here too, because it reads the same signal: it names
//! where the reader is, so it cannot be built from anything but the list the
//! highlight moves along.

import type { Accessor, JSX, Signal } from "solid-js";
import { For, Show, createEffect, createSignal, onCleanup } from "solid-js";

import { Switch } from "../Switch";
import type { Entry, Section, Stands, Watched } from "./outline";
import {
  face,
  inside,
  just,
  lit,
  mark,
  markClass,
  spot,
  standsFor,
} from "./outline";

/// What every part of the nav shares: where the reader is, and the two things
/// about how they got there that a line has to be able to change.
///
/// Passed about as one because all three travel together — a press on a line
/// moves the highlight, pins it, and puts the bar's list away — and because the
/// floating header reads the same `here` the sidebar marks.
export type Nav = {
  /// Where the highlight is, as a place in the watched list.
  here: Signal<number>;

  /// Whether the highlight is being held where a jump put it. Not reactive:
  /// nothing is drawn from it — it only decides whether the spy is the one
  /// saying where the reader is.
  pinned: { held: boolean };

  /// Whether the bar's list is down. Nothing on a wide viewport reads it: the
  /// sidebar's list is always there, so the bar is the only thing this opens.
  open: Signal<boolean>;
};

/// A page's nav state, as one Set's page holds it.
export function navigation(): Nav {
  return {
    // The highlight starts where a page nobody has scrolled puts it, so the nav
    // is already right and the spy has nothing to correct when it first speaks.
    here: createSignal(lit([])),
    pinned: { held: false },
    // Shut: the bar names where the reader is, and the list is what they ask
    // for on top of that.
    open: createSignal(false),
  };
}

/// The table of contents, drawn from the Set and following the reader down it.
///
/// The outline and the watched list are worked out by the caller and handed to
/// the floating header as well, so the two cannot describe different pages.
export function Contents(props: {
  sections: Section[];
  watched: Watched[];
  nav: Nav;
}): JSX.Element {
  const [open, setOpen] = props.nav.open;

  follow(() => props.watched, props.nav);
  dismiss(props.nav);

  return (
    <nav
      class={open() ? "contents contents-open" : "contents"}
      aria-label="On this page"
    >
      <Bar watched={props.watched} nav={props.nav} />
      {/* What a tap away from the open list lands on. It is here rather than a
          listener watching for presses elsewhere so that the tap hits *this*
          and nothing else: a reader taking the list back is not also choosing
          an Option or folding a file, and on a page whose whole purpose is
          answering carefully, a stray tap that answers something is not a small
          thing. Hidden from assistive tech, which has Escape and the button's
          own state instead. */}
      <Show when={open()}>
        <div
          class="contents-backdrop"
          aria-hidden="true"
          onClick={() => setOpen(false)}
        />
      </Show>
      <ol class="contents-sections" id="contents-list">
        <For each={props.sections}>
          {(section) => (
            <li class="contents-section">
              <Link
                anchor={section.anchor}
                label={null}
                text={section.name}
                nav={props.nav}
                stands={standsFor(props.watched, section)}
              />
              <Show when={section.entries.length > 0}>
                <ol class="contents-entries">
                  <For each={section.entries}>
                    {(line) => (
                      <ContentsEntry
                        entry={line}
                        watched={props.watched}
                        nav={props.nav}
                      />
                    )}
                  </For>
                </ol>
              </Show>
            </li>
          )}
        </For>
      </ol>
    </nav>
  );
}

/// One nested line of the nav.
function ContentsEntry(props: {
  entry: Entry;
  watched: Watched[];
  nav: Nav;
}): JSX.Element {
  const at = () => spot(props.watched, props.entry.anchor);
  const stands = (): Stands | null => {
    const place = at();
    return place === null ? null : just(place);
  };

  return (
    // Prefixed like every other class here: `question` on its own is the page's
    // own Question card, and a nav line is not one.
    <li class={`contents-entry ${face(props.entry.label)}`}>
      <Link
        anchor={props.entry.anchor}
        label={props.entry.label}
        text={props.entry.text}
        whole={props.entry.whole}
        nav={props.nav}
        stands={stands()}
      />
    </li>
  );
}

/// The jump itself: an anchor to the id, which script takes over so the jump
/// can unfold what it lands on and leave the history alone.
///
/// It is also where the highlight shows, from what this line stands for: the
/// mark when the reader is at it, and a quieter one on a section while they are
/// somewhere inside it.
function Link(props: {
  anchor: string;
  label: string | null;
  text: string;
  whole?: string;
  nav: Nav;
  stands: Stands | null;
}): JSX.Element {
  const [here, goTo] = props.nav.here;
  const [, setOpen] = props.nav.open;

  const marked = () => mark(props.stands, here());

  const classes = () => {
    const mark = marked();
    return mark === null ? "contents-link" : `contents-link ${markClass(mark)}`;
  };

  return (
    <a
      class={classes()}
      // The nav is a list of places in this page and the highlight says which
      // one the reader is at, which is what `location` means. Only the one line
      // carries it — the section around it is not where they are, it is what
      // they are in.
      aria-current={marked() === "at" ? "location" : undefined}
      href={`#${props.anchor}`}
      title={props.whole}
      onClick={(ev) => {
        // Only the plain click: a modified one is the reader asking their
        // browser for a tab or a window, which is the browser's business and
        // not ours.
        if (ev.ctrlKey || ev.metaKey || ev.shiftKey || ev.altKey) {
          return;
        }
        ev.preventDefault();
        // The highlight goes where the reader asked to be, and is held there
        // until they scroll for themselves: the page cannot always bring a
        // section to the top — the last Question has only the submit below it —
        // and the spy would otherwise answer for wherever the scroll ran out
        // instead of for what was pressed.
        if (props.stands !== null) {
          goTo(props.stands.at);
          props.nav.pinned.held = true;
        }
        // The list has done what it was opened for. On a wide viewport there
        // was no list to put away — the sidebar stays whatever this says.
        setOpen(false);
        jumpTo(props.anchor);
      }}
    >
      <Show when={props.label}>
        {(label) => <span class="contents-label">{label()}</span>}
      </Show>
      {props.text}
    </a>
  );
}

/// The bar: on a narrow viewport, the whole of the nav until it is tapped.
///
/// It says one thing — the name of the line the sidebar would have lit — so the
/// reader knows where they are without a margin to put a list in, and tapping
/// it brings the list itself down. Not drawn at all where the sidebar is, which
/// is the stylesheet's doing: there is only ever one of the two on screen.
///
/// A button rather than a `details`, because the list it opens is the same `ol`
/// the sidebar shows and a closed `details` would have to hide that from the
/// wide reader too.
function Bar(props: { watched: Watched[]; nav: Nav }): JSX.Element {
  const [here] = props.nav.here;
  const [open, setOpen] = props.nav.open;

  // A place the watched list does not have cannot happen — `here` only ever
  // holds one of them — but the bar is not worth a broken page.
  const named = () => props.watched[here()];

  return (
    <button
      type="button"
      class="contents-bar"
      // The bar's own words are its name — "Preface, button" — and the nav
      // around it says what a list of them is for.
      aria-expanded={open() ? "true" : "false"}
      aria-controls="contents-list"
      onClick={() => setOpen(!open())}
    >
      <Show when={named()}>
        {(named) => (
          <span class={`contents-bar-name ${named().kind}`}>
            <Show when={named().label}>
              {(label) => <span class="contents-label">{label()}</span>}
            </Show>
            {named().text}
          </span>
        )}
      </Show>
      {/* Which way the list will go, and no part of what the bar is called. */}
      <span class="contents-bar-mark" aria-hidden="true">
        ▾
      </span>
    </button>
  );
}

/// The floating header: on a wide viewport, where the reader is and — while
/// they are in the Diff — the switch that governs how it is drawn.
///
/// Its own element rather than the nav's bar let through at this width. The bar
/// is a button whose whole job is opening the list, and here the list is
/// already in the margin, so that button would be a control that does nothing —
/// focusable, announced as a button, and inert. They share the signal and
/// nothing else.
///
/// The naming is a breadcrumb rather than one line, because the sidebar beside
/// it already says both halves with two highlights and one line would say less
/// than what it is sitting next to. It reads from the same scroll-spy signal,
/// so it cannot disagree with the sidebar about where the reader is.
///
/// The words are hidden from assistive tech: the sidebar's own line already
/// carries `aria-current`, and a second copy of the answer is a second thing to
/// hear. The switch is not hidden — it is a real control, and the one thing
/// here that is not said somewhere else on screen.
///
/// Nothing to say on a narrow viewport, where the bar is doing this; the
/// stylesheet keeps it off, as it keeps the bar off here.
export function PageHeader(props: {
  watched: Watched[];
  nav: Nav;
  wrapped: boolean;
  flip: (on: boolean) => void;
}): JSX.Element {
  const [here] = props.nav.here;

  const named = () => props.watched[here()];

  // Whether the wrap switch belongs here yet. Read off the section the reader
  // is in rather than off which line exactly, so it stays put as they move from
  // the Diff's heading down through its files instead of flickering per file.
  //
  // A Set with no Diff has no such section, so this is never true for one.
  const inTheDiff = () => named()?.section === "diff";

  return (
    <div class="page-header">
      <p class="page-header-where" aria-hidden="true">
        <Show when={named()}>
          {(named) => (
            <>
              <Show when={inside(named())}>
                {(section) => (
                  <>
                    <span class="page-header-section">{section()}</span>
                    <span class="page-header-sep">›</span>
                  </>
                )}
              </Show>
              <span class={`page-header-name ${named().kind}`}>
                <Show when={named().label}>
                  {(label) => <span class="contents-label">{label()}</span>}
                </Show>
                {named().text}
              </span>
            </>
          )}
        </Show>
      </p>
      <Show when={inTheDiff()}>
        <Switch label="Word wrap" on={props.wrapped} flip={props.flip} />
      </Show>
    </div>
  );
}

/// Take the reader to the section, file or Question this id names.
///
/// A folded file is unfolded before the jump: landing on a closed fold is
/// landing on nothing. The scroll is the browser's own, so how it moves is the
/// stylesheet's business — which is where `prefers-reduced-motion` is honoured.
///
/// The URL is set with `replaceState` rather than by letting the link navigate:
/// the reader gets a hash they can copy or reload into, but moving around a
/// page is not somewhere to come *back* to, and twenty jumps should not bury
/// the list this Set was opened from twenty presses of Back away.
function jumpTo(anchor: string): void {
  const target = document.getElementById(anchor);
  if (target === null) {
    return;
  }

  if (target instanceof HTMLDetailsElement) {
    target.open = true;
  }

  target.scrollIntoView();
  history.replaceState(null, "", `#${anchor}`);
}

/// Where the reading line sits, written as the margins the browser is to put
/// around the window before it decides what is in view.
///
/// The bottom one lifts the line to a tenth of the way down the window: what is
/// under it is what the reader has in front of them, and a section is "started"
/// once its top has passed it. The top one reaches far above the window so that
/// a section long scrolled past still counts as started — `lit` wants the last
/// section to have started, and one that stopped counting from up there was not
/// going to be the last.
const READING_LINE = "100000px 0px -90% 0px";

/// The reader taking the scroll back off a jump: the ways of moving a page that
/// are the human's own rather than something the page did to itself.
///
/// A wheel, a finger or a key, and `pointerdown` for the scrollbar being taken
/// hold of. A press on the nav itself is a `pointerdown` too, which is
/// harmless: the click that follows it pins the highlight again straight after.
const BY_HAND = ["wheel", "pointerdown", "keydown"];

/// Follow the reader down the page: keep `here` on the last of the watched
/// parts to have started above the reading line, which is the one whose text
/// they have in front of them.
///
/// An IntersectionObserver rather than a scroll handler: the browser is the one
/// that knows where everything is, it works this out off the main thread, and
/// it speaks up only when the answer changes. How the page got there does not
/// come into it either, so a reader who asked for no motion gets the highlight
/// an instant jump earns just as a smooth one does — and nothing here touches
/// the URL, because where the reader has scrolled to is not a place they
/// navigated.
///
/// A jump holds the highlight where it landed until the reader scrolls by hand,
/// and letting go of it puts the highlight straight back on wherever the page
/// actually is — which is why what has started is kept out here, where both the
/// observer and the release can read it.
function follow(watched: Accessor<Watched[]>, nav: Nav): void {
  const [, goTo] = nav.here;

  createEffect(() => {
    const parts = watched();

    // A browser with no observer to be had gets a nav whose highlight stays on
    // the first line: a way around the page, minus the one thing that needed
    // watching.
    if (typeof IntersectionObserver !== "function") {
      return;
    }

    // Which of them have started, by their place in `parts`. Kept because the
    // browser reports only what has changed since it last spoke.
    const started: boolean[] = parts.map(() => false);

    const observer = new IntersectionObserver(
      (crossings) => {
        for (const crossing of crossings) {
          const at = spot(parts, crossing.target.id);
          if (at !== null) {
            started[at] = crossing.isIntersecting;
          }
        }

        // Recorded either way, so that letting go of a pin has the truth to
        // hand rather than having to wait for the next crossing.
        if (!nav.pinned.held) {
          goTo(lit(started));
        }
      },
      { rootMargin: READING_LINE },
    );

    // A section the page does not have is skipped rather than fatal: the nav is
    // drawn from the Set and so is this list, but a highlight is not worth
    // dropping the rest of the page over.
    for (const part of parts) {
      const section = document.getElementById(part.anchor);
      if (section !== null) {
        observer.observe(section);
      }
    }

    // The reader taking over: the pin goes, and the highlight catches up to
    // where the page is in the same breath, since nothing may cross the reading
    // line for a while yet.
    const byHand = () => {
      if (nav.pinned.held) {
        nav.pinned.held = false;
        goTo(lit(started));
      }
    };

    for (const moved of BY_HAND) {
      window.addEventListener(moved, byHand);
    }

    // An observer still watching a page that has gone is a scroll away from
    // moving a highlight nobody can see.
    onCleanup(() => {
      observer.disconnect();
      for (const moved of BY_HAND) {
        window.removeEventListener(moved, byHand);
      }
    });
  });
}

/// Put the bar's list away on Escape.
///
/// The other way out of it — tapping the page — is the backdrop's doing rather
/// than a listener's, so that the tap taking the list back cannot also press
/// something on the page underneath. This is the way out that needs no aim: a
/// list drawn over the page has to be dismissible from the keyboard, and there
/// is nothing to tab to that would do it.
function dismiss(nav: Nav): void {
  const [, setOpen] = nav.open;

  const escape = (ev: KeyboardEvent) => {
    if (ev.key === "Escape") {
      setOpen(false);
    }
  };

  document.addEventListener("keydown", escape);
  onCleanup(() => document.removeEventListener("keydown", escape));
}
