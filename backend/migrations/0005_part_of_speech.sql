-- Part of speech for each card, picked from a list next to the word:
-- 'n', 'v', 'adj', 'adv', 'prep', 'conj', 'phrv' (phrasal verb),
-- 'phrase' or 'idiom'. '' = not set.

ALTER TABLE cards ADD COLUMN part_of_speech TEXT NOT NULL DEFAULT '';

-- Before this column existed the part of speech was typed into the word,
-- e.g. "nostalgia (n.)". Move a trailing note like that into the new column
-- and take it off the word.
UPDATE cards
   SET part_of_speech = s.pos,
       word = rtrim(substr(cards.word, 1, length(cards.word) - length(s.note)))
  FROM (
        SELECT '(n.)' AS note, 'n' AS pos
        UNION ALL SELECT '(n)', 'n'
        UNION ALL SELECT '(noun)', 'n'
        UNION ALL SELECT '(v.)', 'v'
        UNION ALL SELECT '(v)', 'v'
        UNION ALL SELECT '(verb)', 'v'
        UNION ALL SELECT '(adj.)', 'adj'
        UNION ALL SELECT '(adj)', 'adj'
        UNION ALL SELECT '(adjective)', 'adj'
        UNION ALL SELECT '(adv.)', 'adv'
        UNION ALL SELECT '(adv)', 'adv'
        UNION ALL SELECT '(adverb)', 'adv'
       ) AS s
 WHERE cards.word LIKE '%' || s.note
   AND length(trim(cards.word)) > length(s.note);
