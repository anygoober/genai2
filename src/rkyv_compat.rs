// This is shown in the example
// https://docs.rs/rkyv/latest/rkyv/trait.Archive.html#examples
//
// I think we'll only need Serialize only.
// Deserializing is basically leaking memory lol

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct JsonValue(pub serde_json::Value);

impl From<serde_json::Value> for JsonValue {
	fn from(v: serde_json::Value) -> Self {
		Self(v)
	}
}
impl From<JsonValue> for serde_json::Value {
	fn from(v: JsonValue) -> Self {
		v.0
	}
}

impl std::ops::Deref for JsonValue {
	type Target = serde_json::Value;
	fn deref(&self) -> &serde_json::Value {
		&self.0
	}
}

impl std::ops::DerefMut for JsonValue {
	fn deref_mut(&mut self) -> &mut serde_json::Value {
		&mut self.0
	}
}

#[cfg(feature = "rkyv")]
mod archive_json_value {
	use super::*;

	// [AI CONTENT]
	pub struct JsonResolver {
		json: String,
		inner: rkyv::string::StringResolver,
	}

	impl rkyv::Archive for JsonValue {
		type Archived = rkyv::string::ArchivedString;
		type Resolver = JsonResolver;

		fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
			rkyv::string::ArchivedString::resolve_from_str(&resolver.json, resolver.inner, out);
		}
	}

	impl<S> rkyv::Serialize<S> for JsonValue
	where
		S: rkyv::rancor::Fallible + rkyv::ser::Writer + ?Sized,
		S::Error: rkyv::rancor::Source,
	{
		fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
			use rkyv::rancor::Source;
			let json = serde_json::to_string(&self.0).map_err(S::Error::new)?;
			let inner = rkyv::string::ArchivedString::serialize_from_str(&json, serializer)?;
			Ok(JsonResolver { json, inner })
		}
	}

	impl<D> rkyv::Deserialize<JsonValue, D> for rkyv::string::ArchivedString
	where
		D: rkyv::rancor::Fallible + ?Sized,
		D::Error: rkyv::rancor::Source,
	{
		fn deserialize(&self, _: &mut D) -> Result<JsonValue, D::Error> {
			use rkyv::rancor::Source;
			serde_json::from_str(self.as_str()).map(JsonValue).map_err(D::Error::new)
		}
	}
	// [/AI CONTENT]
}
