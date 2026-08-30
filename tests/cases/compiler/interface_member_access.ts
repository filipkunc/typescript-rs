// @reference bootstrap/member-access/interface-members

interface Formatter {
    prefix: string;
    format(value: string): string;
}

declare const formatter: Formatter;

const prefix: string = formatter.prefix;
const formatted: string = formatter.format("value");

const wrongProperty: number = formatter.prefix;
formatter.format(1);
formatter.format();
formatter.missing;
