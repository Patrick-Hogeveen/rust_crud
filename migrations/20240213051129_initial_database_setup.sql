-- Add migration script here
create table if not exists recipe
(
    id uuid primary key,
    rec_name text not null,
    inserted_at TIMESTAMPTZ not null,
    updated_at TIMESTAMPTZ not null
);

create table if not exists ingredient (
    id uuid primary key,
    name text not null
);

create table if not exists recipe_ingredients (
    amount double precision not null,
    unit text,
    recid uuid,
    indid uuid,
    foreign key(recid) references recipe(id),
    foreign key(indid) references ingredient(id),
    primary key (recid, indid)
);


--create table "user"
--(
--    user_id       uuid primary key default gen_random_uuid(),
--    username      text unique not null,
--    password_hash text        not null
--);