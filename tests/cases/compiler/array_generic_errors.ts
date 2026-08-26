// @reference: bootstrap/array-generic-syntax-errors
type User = { id: number; name: string };

const numbers: Array<number> = [1, "two"];
const users: Array<User> = [{ id: 1 }, { id: 2, name: false }];
