// @reference: bootstrap/literal-union-aliases
type Status = "open" | "closed";
type Result = Status | "failed";

const open: Status = "open";
const failed: Result = "failed";
const pending: Status = "pending";
const wrongLiteral: "open" = "closed";

type Enabled = true;
const enabled: Enabled = true;
const disabled: Enabled = false;

type Forward = Later | "future";
type Later = "later";
const later: Forward = "later";
const missing: Forward = "missing";

type Port = 80 | 443;
const https: Port = 443;
const ssh: Port = 22;

type Bit = 0n | 1n;
const one: Bit = 1n;
const two: Bit = 2n;
