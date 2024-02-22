use axum::{extract, extract::{State}, http};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

fn uuids(id: uuid::Uuid, size: usize) -> Vec<uuid::Uuid> {
    let mut id_vec: Vec<uuid::Uuid> = Vec::with_capacity(size);
    for i in 0..size {
        id_vec.push(id);
    }

    return id_vec
}

#[derive(Serialize, FromRow, Debug)]
pub struct Recipe {
    id: uuid::Uuid,
    rec_name: String,
    inserted_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl Recipe {
    fn new(name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            rec_name: name,
            inserted_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Deserialize, FromRow, Serialize)]
pub struct Ingredient {
    id: uuid::Uuid,
    name: String,
}

impl Ingredient {
    fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
        }
    }
}

#[derive(Debug, Deserialize, Clone, FromRow, Serialize)]
pub struct RecipeIngredient {
    amount: f64,
    unit: String,
    recid: uuid::Uuid,
    indid: uuid::Uuid,
}

impl RecipeIngredient {
    fn new(amount:f64, unit: String, recipe: uuid::Uuid, ingredient: uuid::Uuid) -> Self {
        Self {
            amount,
            unit,
            recid: recipe,
            indid: ingredient,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InpIngredient {
    name: String,
    amount: f64,
    unit: String,
}

#[derive(Debug, Deserialize)]
pub struct InpRecipe {
    name: String,
    ingredients: Vec<InpIngredient>,
}


pub async fn health() -> http::StatusCode {
    http::StatusCode::OK
}

#[derive(Deserialize)]
pub struct RecId {
    id: uuid::Uuid,
}

pub async fn create_recipe(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<InpRecipe>,
) -> Result<(http::StatusCode, axum::Json<Recipe>), http::StatusCode> {
    let recipe = Recipe::new(payload.name.clone());
    


    println!("{:?}", payload);

    let res_recipe = sqlx::query(
        r#"
            INSERT INTO recipe (id, rec_name, inserted_at, updated_at)
            VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(&recipe.id)
    .bind(&recipe.rec_name)
    .bind(&recipe.inserted_at)
    .bind(&recipe.updated_at)
    .execute(&pool)
    .await;
    let (mut indid, mut name, mut amount, mut unit ): (Vec<uuid::Uuid>, Vec<String>, Vec<f64>, Vec<String>) = (Vec::new(),Vec::new(),Vec::new(),Vec::new());

    for i in 0..payload.ingredients.len() {
        indid.push(uuid::Uuid::new_v4());
        name.push(payload.ingredients[i].name.clone());
        amount.push(payload.ingredients[i].amount);
        unit.push(payload.ingredients[i].unit.clone());
    }
    
    let res_recipe_ingredient = sqlx::query!(
        "INSERT INTO ingredient (id, name) SELECT * FROM UNNEST($1::uuid[], $2::text[])",
        &indid[..],
        &name[..]
    )
    .execute(&pool)
    .await
    .unwrap();
    

    let id_vec = uuids(recipe.id, amount.len());
    let res_ingredient = sqlx::query!(
        "INSERT INTO recipe_ingredients (amount, unit, recid, indid) SELECT * FROM UNNEST($1::double precision[], $2::text[], $3::uuid[], $4::uuid[])",
        &amount[..],
        &unit[..],
        &id_vec,
        &indid[..]
    )
    .execute(&pool)
    .await
    .unwrap();

    

    match res_recipe {
        Ok(_) => Ok((http::StatusCode::CREATED, axum::Json(recipe))),
        Err(_) => Err(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
    
}

pub async fn read_recipes(
    extract::State(pool): extract::State<PgPool>,
) -> Result<axum::Json<Vec<Recipe>>, http::StatusCode> {
    let res = sqlx::query_as::<_, Recipe>("SELECT * FROM recipe")
        .fetch_all(&pool)
        .await;

    println!("{:?}", res);
    match res {
        Ok(recipe) => Ok(axum::Json(recipe)),
        Err(_) => Err(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

//Need func to get ingredients based on recipe id
pub async fn get_ingredients(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<RecId>,
) -> Result<(http::StatusCode, axum::Json<Vec<InpIngredient>>), http::StatusCode> {
    let mut retIngredients = Vec::new();
    let res = sqlx::query_as::<_, RecipeIngredient>("SELECT * FROM recipe_ingredients WHERE recid = $1")
        .bind(payload.id)
        .fetch_all(&pool)
        .await;
    println!("{:?}", res);
    let code: http::StatusCode;
    let mut ingreds = Vec::new();
    match res {
        Ok(ingredient) => {
            ingreds = ingredient;
            code = http::StatusCode::OK;
        },
        Err(_) => {code = http::StatusCode::INTERNAL_SERVER_ERROR},
    };

    for i in 0..ingreds.len() {
        
        let ingredName = sqlx::query_as::<_, Ingredient>("SELECT * FROM ingredient WHERE id = $1")
                .bind(ingreds[i].indid)
                .fetch_all(&pool)
                .await
                .unwrap();
        
        retIngredients.push(
            InpIngredient {
                name: ingredName[0].name.clone(),
                amount: ingreds[i].amount,
                unit: ingreds[i].unit.clone(),
            }
        );
    }   

    
    match code {
        http::StatusCode::OK => Ok((http::StatusCode::OK, axum::Json(retIngredients))),
        (_) => Err(http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(id): extract::Path<uuid::Uuid>,
    axum::Json(payload): axum::Json<InpRecipe>,
) -> http::StatusCode {
    let recipe = Recipe::new(payload.name.clone());
    


    println!("{:?}", payload);

    let res_recipe = sqlx::query(
        r#"
            UPDATE recipe
            SET rec_name =$1, updated_at = $2
            WHERE id = $3
        "#,
    )
    .bind(&recipe.rec_name)
    .bind(&recipe.updated_at)
    .bind(&id)
    .execute(&pool)
    .await;

    removeIngredients(State(pool.clone()), axum::Json(RecId {id: id})).await;

    let (mut indid, mut name, mut amount, mut unit ): (Vec<uuid::Uuid>, Vec<String>, Vec<f64>, Vec<String>) = (Vec::new(),Vec::new(),Vec::new(),Vec::new());

    for i in 0..payload.ingredients.len() {
        indid.push(uuid::Uuid::new_v4());
        name.push(payload.ingredients[i].name.clone());
        amount.push(payload.ingredients[i].amount);
        unit.push(payload.ingredients[i].unit.clone());
    }
    
    let res_recipe_ingredient = sqlx::query!(
        "INSERT INTO ingredient (id, name) SELECT * FROM UNNEST($1::uuid[], $2::text[])",
        &indid[..],
        &name[..]
    )
    .execute(&pool)
    .await
    .unwrap();
    

    let id_vec = uuids(id, amount.len());
    let res_ingredient = sqlx::query!(
        "INSERT INTO recipe_ingredients (amount, unit, recid, indid) SELECT * FROM UNNEST($1::double precision[], $2::text[], $3::uuid[], $4::uuid[])",
        &amount[..],
        &unit[..],
        &id_vec,
        &indid[..]
    )
    .execute(&pool)
    .await
    .unwrap();

    

    match res_recipe {
        Ok(_) => http::StatusCode::CREATED,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn removeIngredients(
    extract::State(pool): extract::State<PgPool>,
    axum::Json(payload): axum::Json<RecId>,
) -> http::StatusCode {
    let id = payload.id;
    println!("HERE: {:?}", id);
    let res = sqlx::query_as::<_, RecipeIngredient>("SELECT * FROM recipe_ingredients WHERE recid = $1")
        .bind(id)
        .fetch_all(&pool)
        .await;
    println!("delete: {:?}", res);
    let code: http::StatusCode;
    let mut ingreds = Vec::new();
    match res {
        Ok(ingredient) => {
            ingreds = ingredient;
            code = http::StatusCode::OK;
        },
        Err(_) => {code = http::StatusCode::INTERNAL_SERVER_ERROR},
    };

    for i in 0..ingreds.len(){
        
        let ingRecRes = sqlx::query(
            r#"
                DELETE FROM recipe_ingredients WHERE indid = $1
            "#
        )
        .bind(ingreds[i].indid)
        .execute(&pool)
        .await;
        
        
    }

    for j in 0..ingreds.len(){
        let ingRes = sqlx::query(
            r#"
                DELETE FROM ingredient WHERE id = $1
            "#
        )
        .bind(ingreds[j].indid)
        .execute(&pool)
        .await;
        
    }

    return code
}

pub async fn delete_recipe(
    extract::State(pool): extract::State<PgPool>,
    extract::Path(id): extract::Path<uuid::Uuid>,
) -> http::StatusCode {
    let recId = RecId{id: id};

    removeIngredients(State(pool.clone()), axum::Json(recId)).await;

    let res = sqlx::query(
        r#"
            DELETE FROM recipe
            WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await
    .map(|res| match res.rows_affected() {
        0 => http::StatusCode::NOT_FOUND,
        _ => http::StatusCode::OK,
    });

    match res {
        Ok(status) => status,
        Err(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}