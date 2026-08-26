// @reference: bootstrap/negative-numeric-literals
type Temperature = -5 | 0 | 5;

const exact: -5 = -5;
const readings: number[] = [-12, 0, 3.5];
const allowed: Temperature = -5;
const wrong: Temperature = -4;
