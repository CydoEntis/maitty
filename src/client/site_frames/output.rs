//! What the made-up agents wrote, as their own screens draw it: SGR colours, CRLF line ends,
//! lines kept under 40 columns so they read whole in a pane of a 120-column window.

const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const OFF: &str = "\x1b[0m";

fn screen(lines: &[String]) -> String {
    lines.join("\r\n")
}

/// Claude asking to run the tests, a turn in. `cols` wide, so its rule spans the pane.
pub(super) fn claude_asks(cols: u16) -> String {
    let rule = "─".repeat(cols.saturating_sub(4) as usize);
    screen(&[
        format!("{DIM}>{OFF} discount codes don't change the total"),
        String::new(),
        format!("● {BOLD}Read{OFF}(src/checkout/total.ts)"),
        "  ⎿  Read 64 lines".into(),
        format!("● {BOLD}Update{OFF}(src/checkout/total.ts)"),
        "  ⎿  5 additions, 1 removal".into(),
        format!("     {RED}26 - return subtotal + shipping;{OFF}"),
        format!("     {GREEN}26 + const off = discount(code);{OFF}"),
        format!("     {GREEN}27 + return subtotal - off + ship;{OFF}"),
        String::new(),
        "● Now the tests, to be sure.".into(),
        String::new(),
        format!("{BLUE}{rule}{OFF}"),
        format!(" {BOLD}{BLUE}Bash command{OFF}"),
        "   pnpm test".into(),
        format!("   {DIM}Run the checkout tests{OFF}"),
        String::new(),
        format!(" {BOLD}Allow Bash(pnpm test)?{OFF}"),
        format!(" {BLUE}❯ 1. Yes{OFF}"),
        "   2. Yes, don't ask again".into(),
        "   3. No, tell Claude what to do".into(),
    ])
}

/// Claude done writing a page.
pub(super) fn claude_done() -> String {
    screen(&[
        format!("{DIM}>{OFF} write a quickstart page"),
        String::new(),
        format!("● {BOLD}Write{OFF}(docs/quickstart.md)"),
        "  ⎿  Wrote 48 lines".into(),
        String::new(),
        "● It covers install, a first order".into(),
        "  and the test card.".into(),
        String::new(),
        format!("{DIM}>{OFF} "),
    ])
}

/// Codex part way through covering the cart with tests.
pub(super) fn codex_working() -> String {
    screen(&[
        format!("{DIM}› {OFF}{BOLD}cover the cart total with tests{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}{BOLD}Explored{OFF}"),
        format!("{DIM}  └ {OFF}Read {CYAN}src/cart/total.ts{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}{BOLD}Edit {OFF}{CYAN}src/cart/total.test.ts{OFF}"),
        format!("{GREEN}  + it(\"totals an empty cart\", () =>{OFF}"),
        format!("{GREEN}  +   expect(total([])).toBe(0));{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}Ran {BLUE}pnpm vitest run cart{OFF}"),
        format!("{DIM}  └ 14 passed, 1 failed{OFF}"),
        String::new(),
        format!("{YELLOW}• {OFF}Empty carts give undefined;"),
        "  fixing the reducer's start.".into(),
        String::new(),
        format!("{DIM}◦ Working (41s · esc to interrupt){OFF}"),
    ])
}

/// Codex finished, waiting for the next ask.
pub(super) fn codex_done() -> String {
    screen(&[
        format!("{DIM}› {OFF}{BOLD}cover the cart total with tests{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}Ran {BLUE}pnpm vitest run cart{OFF}"),
        format!("{DIM}  └ 15 passed{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}Done. Empty carts total 0 now."),
        String::new(),
        format!("{DIM}Worked for 1m 12s{OFF}"),
        String::new(),
        format!("› {DIM}Ask Codex to do anything{OFF}"),
    ])
}

/// Codex writing the shipping email.
pub(super) fn codex_emails() -> String {
    screen(&[
        format!("{DIM}› {OFF}{BOLD}email the buyer when it ships{OFF}"),
        String::new(),
        format!("{GREEN}• {OFF}{BOLD}Edit {OFF}{CYAN}src/orders/events.ts{OFF}"),
        format!("{GREEN}  + on(\"shipped\", sendShippedMail);{OFF}"),
        format!("{GREEN}• {OFF}{BOLD}Add {OFF}{CYAN}emails/shipped.tsx{OFF}"),
        String::new(),
        format!("{DIM}◦ Working (12s · esc to interrupt){OFF}"),
    ])
}

/// Gemini done adding the rate limit.
pub(super) fn gemini_done() -> String {
    screen(&[
        format!("{MAGENTA}> {OFF}add a per-key rate limit"),
        String::new(),
        format!("{GREEN}✓ {OFF}{BOLD}WriteFile{OFF} src/limit/bucket.go"),
        format!("{GREEN}✓ {OFF}{BOLD}Edit{OFF} src/proxy.go"),
        format!("{GREEN}✓ {OFF}{BOLD}Shell{OFF} go test ./... {DIM}(ok){OFF}"),
        String::new(),
        format!("{MAGENTA}✦ {OFF}Each key gets 100 requests a"),
        "  minute, then a 429 with".into(),
        "  Retry-After.".into(),
    ])
}

/// OpenCode at rest.
pub(super) fn opencode_idle() -> String {
    screen(&[
        format!("{BOLD}opencode{OFF}"),
        String::new(),
        format!("{DIM}/new      new session{OFF}"),
        format!("{DIM}/models   pick a model{OFF}"),
        String::new(),
        format!("{CYAN}>{OFF} "),
    ])
}
