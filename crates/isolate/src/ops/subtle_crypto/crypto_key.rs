use std::rc::Rc;

use anyhow::Context as _;
use deno_core::v8;
use indexmap::IndexSet;
use slab::Slab;

use super::{
    aes,
    ec,
    ed25519,
    hmac,
    pbkdf2,
    rsa,
    x25519,
    KeyType,
    KeyUsage,
};
use crate::{
    convert_v8::{
        DOMException,
        DOMExceptionName,
        FromV8,
        ToV8,
    },
    strings,
};

#[derive(Default)]
struct CryptoKeyStore {
    keys: Slab<(v8::Weak<v8::Object>, Rc<CryptoKey>)>,
}

pub(super) enum CryptoKeyKind {
    Pbkdf2 {
        algorithm: pbkdf2::Pbkdf2Algorithm,
        key: pbkdf2::Pbkdf2Key,
    },
    Hmac {
        algorithm: hmac::HmacKeyAlgorithm,
        key: hmac::HmacKey,
    },
    Aes {
        algorithm: aes::AesKeyAlgorithm,
        key: aes::AesKey,
    },
    RsaPrivate {
        algorithm: rsa::RsaHashedKeyAlgorithm,
        key: rsa::RsaPrivateKey,
    },
    RsaPublic {
        algorithm: rsa::RsaHashedKeyAlgorithm,
        key: rsa::RsaPublicKey,
    },
    EcPrivate {
        algorithm: ec::EcKeyAlgorithm,
        key: ec::EcPrivateKey,
    },
    EcPublic {
        algorithm: ec::EcKeyAlgorithm,
        key: ec::EcPublicKey,
    },
    Ed25519Private {
        algorithm: ed25519::Ed25519Algorithm,
        key: ed25519::Ed25519PrivateKey,
    },
    Ed25519Public {
        algorithm: ed25519::Ed25519Algorithm,
        key: ed25519::Ed25519PublicKey,
    },
    X25519Private {
        algorithm: x25519::X25519Algorithm,
        key: x25519::X25519PrivateKey,
    },
    X25519Public {
        algorithm: x25519::X25519Algorithm,
        key: x25519::X25519PublicKey,
    },
}

pub(super) struct CryptoKey {
    pub kind: CryptoKeyKind,
    pub r#type: KeyType,
    pub extractable: bool,
    pub usages: IndexSet<KeyUsage>,
}

fn get_crypto_key_template<'s>(
    scope: &mut v8::PinScope<'s, '_>,
) -> anyhow::Result<v8::Local<'s, v8::FunctionTemplate>> {
    let s = strings::CryptoKey.create(scope)?;
    let private = v8::Private::for_api(scope, Some(s));
    let obj: v8::Local<'_, v8::Object> = scope
        .get_current_context()
        .global(scope)
        .get_private(scope, private)
        .context("missing CryptoKey private")?
        .try_cast()?;
    let template: v8::Local<'_, v8::FunctionTemplate> = obj
        .get_internal_field(scope, 0)
        .context("internal field missing")?
        .try_cast()?;
    Ok(template)
}

// Looks up a Rust CryptoKey in the CryptoKeyStore based on the passed-in
// CryptoKey instance.
impl FromV8 for CryptoKey {
    type Output = Rc<Self>;

    fn from_v8<'s>(
        scope: &mut v8::PinScope<'s, '_>,
        input: v8::Local<'s, v8::Value>,
    ) -> anyhow::Result<Rc<Self>> {
        let crypto_key = get_crypto_key_template(scope)?;
        let crypto_key_constructor = crypto_key
            .get_function(scope)
            .context("get CryptoKey constructor")?;
        let input = input.try_cast::<v8::Object>()?;
        anyhow::ensure!(
            input.instance_of(scope, crypto_key_constructor.into()) == Some(true),
            "TypeError"
        );
        let (id, ok) = input
            .get_internal_field(scope, 0)
            .context("missing internal field")?
            .try_cast::<v8::BigInt>()?
            .u64_value();
        anyhow::ensure!(ok);
        let (weak, key) = scope
            .get_slot::<CryptoKeyStore>()
            .context("missing CryptoKeyStore")?
            .keys
            .get(id as usize)
            .context("dangling CryptoKey")?;
        anyhow::ensure!(*weak == input, "CryptoKey id reused");
        Ok(key.clone())
    }
}

// Converts a freshly created CryptoKey instance into a new JS object and
// records it in the CryptoKeyStore so that it can be passed back into crypto
// APIs.
//
// Note that we never need to return a pre-existing CryptoKey instance.
impl ToV8 for CryptoKey {
    fn to_v8<'s>(
        self,
        scope: &mut v8::PinScope<'s, '_>,
    ) -> anyhow::Result<v8::Local<'s, v8::Value>> {
        let crypto_key = get_crypto_key_template(scope)?;
        let object = crypto_key
            .instance_template(scope)
            .new_instance(scope)
            .context("failed to create instance")?;
        anyhow::ensure!(object.internal_field_count() == 1);
        let type_str = strings::r#type.create(scope)?;
        let r#type = self.r#type.to_v8(scope)?;
        anyhow::ensure!(
            object.define_own_property(
                scope,
                type_str.into(),
                r#type,
                v8::PropertyAttribute::READ_ONLY | v8::PropertyAttribute::DONT_DELETE
            ) == Some(true)
        );
        let extractable_str = strings::extractable.create(scope)?;
        let extractable = self.extractable.to_v8(scope)?;
        anyhow::ensure!(
            object.define_own_property(
                scope,
                extractable_str.into(),
                extractable,
                v8::PropertyAttribute::READ_ONLY | v8::PropertyAttribute::DONT_DELETE
            ) == Some(true)
        );
        let algorithm_str = strings::algorithm.create(scope)?;
        // TODO: the resulting `algorithm` object has a null prototype, which
        // looks ugly when inspected.
        let algorithm = match &self.kind {
            CryptoKeyKind::Pbkdf2 { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::Hmac { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::Aes { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::RsaPrivate { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::RsaPublic { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::EcPrivate { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::EcPublic { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::Ed25519Private { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::Ed25519Public { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::X25519Private { algorithm, .. } => algorithm.to_v8(scope)?,
            CryptoKeyKind::X25519Public { algorithm, .. } => algorithm.to_v8(scope)?,
        };
        anyhow::ensure!(
            object.define_own_property(
                scope,
                algorithm_str.into(),
                algorithm,
                v8::PropertyAttribute::READ_ONLY | v8::PropertyAttribute::DONT_DELETE
            ) == Some(true)
        );
        let usages_str = strings::usages.create(scope)?;
        let usages = (&self.usages).to_v8(scope)?;
        anyhow::ensure!(
            object.define_own_property(
                scope,
                usages_str.into(),
                usages,
                v8::PropertyAttribute::READ_ONLY | v8::PropertyAttribute::DONT_DELETE
            ) == Some(true)
        );
        let mut store: CryptoKeyStore = scope.remove_slot().unwrap_or_default();
        let entry = store.keys.vacant_entry();
        let id = entry.key();
        assert!(object.set_internal_field(0, v8::BigInt::new_from_u64(scope, id as u64).into()));
        // Install a finalizer so that we garbage collect the Rust data together
        // with the JS object.
        // This does not need to be a "guaranteed finalizer" because the data is
        // also dropped when the Isolate is destroyed.
        let weak = v8::Weak::with_finalizer(
            scope,
            object,
            Box::new(move |isolate| {
                if let Some(store) = isolate.get_slot_mut::<CryptoKeyStore>() {
                    store.keys.remove(id);
                }
            }),
        );
        entry.insert((weak, Rc::new(self)));
        scope.set_slot(store);
        Ok(object.into())
    }
}

impl CryptoKey {
    pub(super) fn check_usage(&self, usage: KeyUsage) -> anyhow::Result<()> {
        if !self.usages.contains(&usage) {
            anyhow::bail!(DOMException::new(
                format!(
                    "CryptoKey does not have {} usage",
                    serde_json::to_string(&usage)?
                ),
                DOMExceptionName::InvalidAccessError
            ));
        }
        Ok(())
    }

    /// If the [[type]] internal slot of result is "secret" or "private" and
    /// usages is empty, then throw a SyntaxError.
    pub(super) fn check_useless(&self) -> anyhow::Result<()> {
        if [KeyType::Secret, KeyType::Private].contains(&self.r#type) && self.usages.is_empty() {
            anyhow::bail!(DOMException::new(
                "invalid key usages",
                DOMExceptionName::SyntaxError,
            ));
        }
        Ok(())
    }
}

pub(super) struct CryptoKeyPair {
    pub private_key: CryptoKey,
    pub public_key: CryptoKey,
}

impl ToV8 for CryptoKeyPair {
    fn to_v8<'s>(
        self,
        scope: &mut v8::PinScope<'s, '_>,
    ) -> anyhow::Result<v8::Local<'s, v8::Value>> {
        // N.B.: a CryptoKeyPair is just a regular object, not a special class
        let private_key = self.private_key.to_v8(scope)?;
        let public_key = self.public_key.to_v8(scope)?;
        let crypto_key_pair = v8::Object::new(scope);
        let private_key_str = strings::privateKey.create(scope)?;
        anyhow::ensure!(
            crypto_key_pair.set(scope, private_key_str.into(), private_key) == Some(true)
        );
        let public_key_str = strings::publicKey.create(scope)?;
        anyhow::ensure!(
            crypto_key_pair.set(scope, public_key_str.into(), public_key) == Some(true)
        );
        Ok(crypto_key_pair.into())
    }
}

pub(super) enum CryptoKeyOrPair {
    Symmetric(CryptoKey),
    Asymmetric(CryptoKeyPair),
}

impl From<CryptoKeyPair> for CryptoKeyOrPair {
    fn from(v: CryptoKeyPair) -> Self {
        Self::Asymmetric(v)
    }
}

impl From<CryptoKey> for CryptoKeyOrPair {
    fn from(v: CryptoKey) -> Self {
        Self::Symmetric(v)
    }
}

impl ToV8 for CryptoKeyOrPair {
    fn to_v8<'s>(
        self,
        scope: &mut v8::PinScope<'s, '_>,
    ) -> anyhow::Result<v8::Local<'s, v8::Value>> {
        match self {
            CryptoKeyOrPair::Symmetric(k) => k.to_v8(scope),
            CryptoKeyOrPair::Asymmetric(kp) => kp.to_v8(scope),
        }
    }
}
