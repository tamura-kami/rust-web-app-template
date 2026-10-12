CREATE TABLE books (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200)
);
-- 公開サンプルデータのみ。更新 API は認証方針の決定後に追加する。
INSERT INTO books (title) VALUES ('Rust Web API sample');
