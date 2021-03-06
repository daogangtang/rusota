
use crate::db;
use crate::util::{random_string, sha3_256_encode, make_pwd_encode};
use redis::Commands;
use chrono::{DateTime, Utc};

use log::info;
use uuid::Uuid;

pub use crate::model::Ruser;

pub fn set_session(account: &str, ttl: usize) -> Result<String, String> {
    let redis = db::get_redis();
    let cookie = sha3_256_encode(&random_string(8));
    let _: () = redis.hset(&cookie, "login_time", Utc::now().timestamp()).unwrap();
    let _: () = redis.hset(&cookie, "account", account).unwrap();
    let _: () = redis.expire(&cookie, ttl).unwrap();

    Ok(cookie)
}



/// ===== Struct Definition =====
// these structs are defined for request params
pub struct UserSignUp {
    pub account: String,
    pub password: String,
    pub nickname: String,
}

pub struct UserLogin {
    pub account: String,
    pub password: String,
}

#[derive(Debug)]
pub struct GithubUserInfo {
    pub account: String,
    pub github_address: String,
}

pub use crate::model::for_write::{
    UserCreate,
    UserEdit,
    UpdateUserNickname,
    UserChangePassword,
    SectionCreate,
};

pub use crate::model::for_read::{
    RuserWithoutPwd,
};


/// ===== Implementation Area =====
///
impl UserSignUp {
    pub fn sign_up(&self, github_home: Option<String>) -> Result<String, String>{
        let em = db::get_db();
        let salt = random_string(6);

        let new_user = UserCreate {
            account: self.account.to_owned(),
            password: make_pwd_encode(&self.password, &salt),
            salt: salt,
            nickname: self.nickname.to_owned(),
            github: github_home,
        };

        let rest_clause = format!("WHERE account='{}'", new_user.account);
        // check if the same name account exists already
        match db_find!(em, "", "", &rest_clause, Ruser) {
            Some(_) => {
                // exist already, return Error
                Err(format!("user {} exists.", new_user.account))
            },
            None => {
                // it's a new user, insert it
                match db_insert!(em, &new_user, Ruser) {
                    Some(user) => {
                        // generate a corresponding section to this user as his blog section
                        let section = SectionCreate {
                            title: user.nickname.to_owned(),
                            description: format!("{}'s blog", user.nickname),
                            stype: 1,
                            suser: Some(user.id.to_owned()),
                        };
                        section.insert().unwrap();

                        Ok("register success.".to_string())
                        //let ttl = 60*24*3600;
                        // set user cookies to redis to keep login session
                        //set_session(&user.account, ttl)
                    },
                    None => {
                        unreachable!();
                    }
                }
            }
        }
    }
}

impl UserLogin {

    pub fn verify_login(&self) -> Result<String, String> {
        let em = db::get_db();

        let rest_clause = format!("WHERE status=0 and account='{}'", self.account);
        // check if the same name account exists already
        match db_find!(em, "", "", &rest_clause, Ruser) {
            Some(user) => {
                // check calulation equality
                if user.password == make_pwd_encode(&self.password, &user.salt) {
                    let ttl = 60*24*3600;

                    // store session
                    set_session(&self.account, ttl)

                } else {
                    Err("Wrong account or password.".into())
                }

            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }

    pub fn verify_login_with_rawpwd(&self) -> Result<String, String> {
        let em = db::get_db();

        let rest_clause = format!("WHERE status=0 and account='{}'", self.account);
        // check if the same name account exists already
        match db_find!(em, "", "", &rest_clause, Ruser) {
            Some(user) => {
                // check calulation equality
                if user.password == self.password {
                    let ttl = 60*24*3600;

                    // store session
                    set_session(&self.account, ttl)

                } else {
                    Err("Wrong account or password.".into())
                }

            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }
}


impl UserEdit {
    pub fn update(&self, cookie: &str) -> Result<Ruser, String> {
        let em = db::get_db();
        let redis = db::get_redis();
        let account: String = redis.hget(cookie, "account").unwrap();

        // update new info by account
        let clause = format!("WHERE account='{}'", account);
        match db_update!(em, self, &clause, Ruser) {
            Some(user) => {
                Ok(user.to_owned())
            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }
}

impl UpdateUserNickname {
    pub fn update(&self) -> Result<Ruser, String> {
        let em = db::get_db();

        // update new info by account
        let clause = format!("WHERE id='{}'", self.id);
        match db_update!(em, self, &clause, Ruser) {
            Some(user) => {
                Ok(user.to_owned())
            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }
}

impl UserChangePassword {
    pub fn change(&self) -> Result<Ruser, String> {
        let em = db::get_db();

        let clause = format!("WHERE id='{}'", self.id);
        match db_update!(em, self, &clause, Ruser) {
            Some(user) => {
                Ok(user.to_owned())
            },
            None => {
                Err("User doesn't exist.".into())
            }
        }
    }
}


impl Ruser {
    pub fn get_user_by_cookie(cookie: &str) -> Result<Ruser, String> {
        let redis = db::get_redis();
        let account_r: Result<String, _> = redis.hget(cookie, "account");
        match account_r {
            Ok(account) => {
                let clause = format!("where account='{}'", account);
                let em = db::get_db();
                match db_find!(em, "", "", &clause, Ruser) {
                    Some(user) => {
                        Ok(user)
                    },
                    None => Err("no this user".to_string())
                }
            
            },
            Err(_) => {
                Err("no cookie cached".to_string())
            }
        }
    }

    pub fn get_user_by_account(account: &str) -> Result<Ruser, String> {
        let em = db::get_db();
        let clause = format!("where account='{}'", account);
        match db_find!(em, "", "", &clause, Ruser) {
            Some(user) => {
                Ok(user)
            },
                None => Err("no this user".to_string())
        }
    }

    pub fn get_user_by_id(id: Uuid) -> Result<Ruser, String> {
        let em = db::get_db();
        let clause = format!("where id='{}'", id);
        match db_find!(em, "", "", &clause, Ruser) {
            Some(user) => {
                Ok(user)
            },
                None => Err("no this user".to_string())
        }
    }

    pub fn sign_out(cookie: &str) -> Result<(), String> {
        let redis = db::get_redis();
        let _: () = redis.del(cookie).unwrap();

        Ok(())
    }

}

