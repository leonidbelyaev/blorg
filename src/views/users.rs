#[derive(FromForm, Deserialize)]
struct UserInfo {
    username: String,
    password: String
}

#[post("/users/create", format="json", data="<user_info>")]
fn create(user_info: Json<UserInfo>)
  -> Json<i32> {
    let new_user = User
        { 
            username: user_info.username.clone(),
            password_hash: 
        };
    let connection = ...;
    let user_entity: UserEntity = diesel::insert_into(users::table)...
    …
}

fn hash_password(password: &String) -> String {
    let mut hasher = Sha3::sha3_256();
    hasher.input_str(password);
    hasher.result_str()
}
