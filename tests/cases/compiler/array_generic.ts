// @reference: bootstrap/array-generic-syntax
type User = { id: number; name: string };
type Groups = Array<Array<User>>;

const numbers: Array<number> = [1, 2, 3];
const users: Array<User> = [{ id: 1, name: "Ada" }];
const groups: Groups = [[{ id: 2, name: "Grace" }]];
const letters: Array<"a" | "b"> = ["a", "b"];
