-- The AI call log: what went over the wire on every call to the gateway,
-- so a surprising or broken reply can be looked at later.
--
-- Calls that fail before any feedback (quota, network, rejected key) now get
-- a row too, with 0 tokens and the reason in `error`. Rows from before this
-- migration have NULL in every new column.
--
-- Never stored: the API key (it travels in a header, not the body) and image
-- data (the request shows a short description with the image's SHA-256).

ALTER TABLE ai_feedback ADD COLUMN request_json TEXT;   -- last request body sent
ALTER TABLE ai_feedback ADD COLUMN raw_reply    TEXT;   -- the gateway's body as received; NULL = no answer
ALTER TABLE ai_feedback ADD COLUMN http_status  INTEGER;
ALTER TABLE ai_feedback ADD COLUMN duration_ms  INTEGER; -- whole call, a retry included
ALTER TABLE ai_feedback ADD COLUMN attempts     INTEGER; -- 2 when the first reply was unusable
ALTER TABLE ai_feedback ADD COLUMN error        TEXT;   -- why there is no feedback; NULL when there is
