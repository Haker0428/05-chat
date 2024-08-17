use crate::User;
use jwt_simple::prelude::*;
use std::ops::Deref;

const JWT_DUTARION: u64 = 60 * 60 * 24 * 7;
const JWT_ISSUER: &str = "chat_server";
const JWT_AUDIENCE: &str = "chat_web";

pub struct EncodingKey(Ed25519KeyPair);

pub struct DecodingKey(Ed25519PublicKey);

impl EncodingKey {
    pub fn load(pem: &str) -> Result<EncodingKey, jwt_simple::Error> {
        Ok(Self(Ed25519KeyPair::from_pem(pem)?))
    }

    pub fn sign(&self, user: impl Into<User>) -> Result<String, jwt_simple::Error> {
        let claims = Claims::with_custom_claims(user.into(), Duration::from_secs(JWT_DUTARION));
        let claim = claims.with_issuer(JWT_ISSUER).with_audience(JWT_AUDIENCE);
        Ok(self.0.sign(claim)?)
    }
}

impl DecodingKey {
    pub fn load(pem: &str) -> Result<Self, jwt_simple::Error> {
        Ok(Self(Ed25519PublicKey::from_pem(pem)?))
    }

    #[allow(unused)]
    pub fn verify(&self, token: &str) -> Result<User, jwt_simple::Error> {
        let mut options = VerificationOptions::default();
        options.allowed_issuers = Some(HashSet::from_strings(&[JWT_ISSUER]));
        options.allowed_audiences = Some(HashSet::from_strings(&[JWT_AUDIENCE]));

        let claims = self.0.verify_token::<User>(token, Some(options))?;
        Ok(claims.custom)
    }
}

#[allow(unused)]
pub fn generate_token(user: User, key: &EncodingKey) -> Result<String, jwt_simple::Error> {
    let claims = Claims::with_custom_claims(user, Duration::from_secs(JWT_DUTARION));
    Ok(key.0.sign(claims)?)
}

impl Deref for EncodingKey {
    type Target = Ed25519KeyPair;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for DecodingKey {
    type Target = Ed25519PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{DecodingKey, EncodingKey, User};

    #[tokio::test]
    async fn jwt_sign_verify_should_work() -> Result<(), jwt_simple::Error> {
        let encoding_pem = include_str!("../../fixtures/encoding.pem");
        let decoding_pem = include_str!("../../fixtures/decoding.pem");

        let ek = EncodingKey::load(encoding_pem)?;
        let dk = DecodingKey::load(decoding_pem)?;

        let user = User::new(1, "HP", "HP@example.com");
        let token = ek.sign(user.clone())?;

        let user2 = dk.verify(&token)?;

        assert_eq!(user, user2);
        Ok(())
    }
}
