// @reference: bootstrap/array-incorrect-element-types
type Scores = number[];

const wrongScore: Scores = [1, "two", 3];
const wrongNested: boolean[][] = [[true], [false, 0]];
