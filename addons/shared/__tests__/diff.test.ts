/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import {diffBlocks, splitLines, collapseContextBlocks} from '../diff';

describe('diffBlocks', () => {
  it('returns a "=" block for unchanged content', () => {
    const lines = splitLines('a\nb\nc\nd\ne\n');
    expect(diffBlocks(lines, lines)).toMatchObject([['=', [0, 5, 0, 5]]]);
  });

  it('returns a "!" block for totally different contents', () => {
    const aLines = splitLines('x\ny\n');
    const bLines = splitLines('a\nb\nc\n');
    expect(diffBlocks(aLines, bLines)).toMatchObject([['!', [0, 2, 0, 3]]]);
  });

  it('returns "= ! =" blocks when a line was changed in the middle', () => {
    const aLines = splitLines('a\nb\nc\nd\ne\n');
    const bLines = splitLines('a\nb\nc\nd1\nd2\ne\n');
    expect(diffBlocks(aLines, bLines)).toMatchObject([
      ['=', [0, 3, 0, 3]],
      ['!', [3, 4, 3, 5]],
      ['=', [4, 5, 5, 6]],
    ]);
  });

  it('matches mdiff.blocks (known good diff algorithm), excluding empty blocks', () => {
    // Test cases are generated by:
    //
    // ```
    // #!sl dbsh
    // import json
    // allblocks = e.mdiff.allblocks
    // cases = []
    // for bits in range(16):
    //     a = ['a\n', 'b\n', 'c\n', 'd\n']
    //     b = [bits & (1 << i) and c.upper() or c for i, c in enumerate(a)]
    //     a = ''.join(a)
    //     b = ''.join(b)
    //     blocks = [[s, l] for l, s in allblocks(a, b) if l[0] < l[1] or l[2] < l[3]] # skip empty blocks
    //     cases.append(json.dumps(blocks).replace(' ', ''))
    // print(' '.join(cases))
    // ```
    //
    // String is used to prettier from wrapping lines.
    const testCaseStr =
      '[["=",[0,4,0,4]]] [["!",[0,1,0,1]],["=",[1,4,1,4]]] [["=",[0,1,0,1]],["!",[1,2,1,2]],["=",[2,4,2,4]]] [["!",[0,2,0,2]],["=",[2,4,2,4]]] [["=",[0,2,0,2]],["!",[2,3,2,3]],["=",[3,4,3,4]]] [["!",[0,1,0,1]],["=",[1,2,1,2]],["!",[2,3,2,3]],["=",[3,4,3,4]]] [["=",[0,1,0,1]],["!",[1,3,1,3]],["=",[3,4,3,4]]] [["!",[0,3,0,3]],["=",[3,4,3,4]]] [["=",[0,3,0,3]],["!",[3,4,3,4]]] [["!",[0,1,0,1]],["=",[1,3,1,3]],["!",[3,4,3,4]]] [["=",[0,1,0,1]],["!",[1,2,1,2]],["=",[2,3,2,3]],["!",[3,4,3,4]]] [["!",[0,2,0,2]],["=",[2,3,2,3]],["!",[3,4,3,4]]] [["=",[0,2,0,2]],["!",[2,4,2,4]]] [["!",[0,1,0,1]],["=",[1,2,1,2]],["!",[2,4,2,4]]] [["=",[0,1,0,1]],["!",[1,4,1,4]]] [["!",[0,4,0,4]]]';
    const testCases: Array<Block[]> = testCaseStr.split(' ').map(s => JSON.parse(s));
    testCases.forEach((expected, bits) => {
      // eslint-disable-next-line no-bitwise
      const hasBit = (i: number): boolean => (bits & (1 << i)) > 0;
      const a = ['a\n', 'b\n', 'c\n', 'd\n'];
      const b = a.map((s, i) => (hasBit(i) ? s.toUpperCase() : s));
      const actual = diffBlocks(a, b);
      expect(actual).toEqual(expected);
    });
  });
});

describe('collapseContextBlocks', () => {
  it('collapses everything in a "=" block', () => {
    expect(collapseContextBlocks([['=', [0, 5, 0, 5]]], () => false)).toMatchObject([
      ['~', [0, 5, 0, 5]],
    ]);
  });

  it('collapses the top part of a "=" block', () => {
    expect(
      collapseContextBlocks(
        [
          ['=', [0, 5, 0, 5]],
          ['!', [5, 6, 5, 7]],
        ],
        () => false,
      ),
    ).toMatchObject([
      ['~', [0, 2, 0, 2]],
      ['=', [2, 5, 2, 5]],
      ['!', [5, 6, 5, 7]],
    ]);
  });

  it('collapses the bottom part of a "=" block', () => {
    expect(
      collapseContextBlocks(
        [
          ['!', [0, 2, 0, 3]],
          ['=', [2, 8, 3, 9]],
        ],
        () => false,
      ),
    ).toMatchObject([
      ['!', [0, 2, 0, 3]],
      ['=', [2, 5, 3, 6]],
      ['~', [5, 8, 6, 9]],
    ]);
  });

  it('splits a "=" block in 3 blocks on demand', () => {
    expect(
      collapseContextBlocks(
        [
          ['!', [0, 1, 0, 2]],
          ['=', [1, 10, 2, 11]],
          ['!', [10, 11, 11, 12]],
        ],
        () => false,
      ),
    ).toMatchObject([
      ['!', [0, 1, 0, 2]],
      ['=', [1, 4, 2, 5]],
      ['~', [4, 7, 5, 8]],
      ['=', [7, 10, 8, 11]],
      ['!', [10, 11, 11, 12]],
    ]);
  });

  it('respects isExpanded function', () => {
    expect(
      collapseContextBlocks(
        [
          ['!', [0, 1, 0, 2]],
          ['=', [1, 10, 2, 11]],
          ['!', [10, 11, 11, 12]],
        ],
        (aLine, _bLine) => aLine === 4,
      ),
    ).toMatchObject([
      ['!', [0, 1, 0, 2]],
      ['=', [1, 10, 2, 11]],
      ['!', [10, 11, 11, 12]],
    ]);
  });

  it('skips "~" if "=" block is too small', () => {
    expect(
      collapseContextBlocks(
        [
          ['!', [0, 1, 0, 2]],
          ['=', [1, 7, 2, 8]],
          ['!', [7, 8, 8, 9]],
        ],
        () => false,
      ),
    ).toMatchObject([
      ['!', [0, 1, 0, 2]],
      ['=', [1, 7, 2, 8]],
      ['!', [7, 8, 8, 9]],
    ]);
  });

  it('preserves context around empty ! block', () => {
    expect(
      collapseContextBlocks(
        [
          ['=', [0, 5, 0, 5]],
          ['!', [5, 5, 5, 5]],
          ['=', [5, 6, 5, 6]],
        ],
        () => false,
      ),
    ).toEqual([
      ['~', [0, 2, 0, 2]],
      ['=', [2, 5, 2, 5]],
      ['!', [5, 5, 5, 5]],
      ['=', [5, 6, 5, 6]],
    ]);
  });

  it('handles adjacent "=" blocks', () => {
    expect(
      collapseContextBlocks(
        [
          ['=', [0, 2, 0, 2]],
          ['=', [2, 8, 2, 8]],
        ],
        () => false,
      ),
    ).toMatchObject([
      ['~', [0, 2, 0, 2]],
      ['~', [2, 8, 2, 8]],
    ]);
  });
});
