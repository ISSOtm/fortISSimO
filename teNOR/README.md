# teNOR

teNOR is fortISSimO's custom song exporter.
It is required because fortISSimO uses a different data format from hUGEDriver.

## Why "teNOR"?

> &lt;ISSOtm&gt; Because tenors sing fortissimo!  
> &lt;ISSOtm&gt; And the capitalisation is just a given for everything revolving around hUGETracker 😜

It doesn't mean anything, but I'm sure someone will come up with a backronym for it.

## Pattern packing

### Problem

Patterns can be overlapped![^why_overlap]
For example, if pattern <var>P</var> ends with row <var>R</var>, and another pattern <var>P'</var> begins with the same row <var>R</var>, then we can store <var>P'</var> right after <var>P</var>, and "chop off" row <var>X</var> from <var>P</var>.

So the question is: what's the way to overlap the patterns that reduces the song size the most?

[^why_overlap]: That said, the reason why pattern overlapping is an option at all is because the order matrix stores *pointers* to the patterns instead of indices, which allows them to point anywhere.

### Algorithm

Time to describe the algorithm!
But first, some trivia: this was brainstormed during an afternoon with a friend of mine (though I must admit he did most of the work).

This algorithm is a combination of gluttonous and dynamic programming.

First, let's reformulate the problem to be a little more tractable:
- "How to overlap the patterns" actually boils down to figuring out the optimal *order* in which to place them!
  Once an order is decided, actually producing the overlapped patterns is trivial.
- We are trying to find the minimal size, which is equivalent to *maximising the number of shared rows*! (Hereafter referred to as their "score".)

So how do we find the order in which to store patterns that maximises the number of shared rows?

We use rows that contain orderings, and their associated "score", those rows containing one cell per pattern.
The following row is computed cell by cell:
- For the new row's cell <var>i</var>,
  - For each cell in the old row whose ordering does not already include pattern <var>i</var>[^duplicate_indices],
    - Compute the score of that ordering with pattern <var>i</var> appended

    ...and keep the ordering with the best score[^no_candidate].

The operation is repeated as many times as there are patterns; then the row contains valid orderings, and we pick the one with the best score.

[^duplicate_indices]: Appending <var>i</var> to an ordering that already contains it makes that ordering invalid. (And besides, duplicating a pattern wouldn't exactly save space, would it?)

[^no_candidate]: It's possible that there are no candidates at all for a cell! Then the cell becomes empty.

## License

Unlike fortISSimO, teNOR is licensed under the Mozilla Public License 2.0.
