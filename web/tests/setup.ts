import { cleanup } from "@solidjs/testing-library";
import { afterEach } from "vitest";

// The library cleans up after itself only when vitest's globals are on, and
// they are not: an uncleaned render leaves the last test's DOM in the document
// for the next one's queries to find two of everything.
afterEach(cleanup);
