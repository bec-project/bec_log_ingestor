#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

#[doc = r" Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
#[doc = "BEC status enum"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"BECStatus\","]
#[doc = "  \"description\": \"BEC status enum\","]
#[doc = "  \"type\": \"integer\","]
#[doc = "  \"enum\": ["]
#[doc = "    2,"]
#[doc = "    1,"]
#[doc = "    0,"]
#[doc = "    -1"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Serialize, Clone, Debug, PartialEq)]
#[serde(transparent)]
pub struct BecStatus(i64);
impl ::std::ops::Deref for BecStatus {
    type Target = i64;
    fn deref(&self) -> &i64 {
        &self.0
    }
}
impl ::std::convert::From<BecStatus> for i64 {
    fn from(value: BecStatus) -> Self {
        value.0
    }
}
impl ::std::convert::From<&BecStatus> for BecStatus {
    fn from(value: &BecStatus) -> Self {
        value.clone()
    }
}
impl ::std::convert::TryFrom<i64> for BecStatus {
    type Error = self::error::ConversionError;
    fn try_from(value: i64) -> ::std::result::Result<Self, self::error::ConversionError> {
        if ![2_i64, 1_i64, 0_i64, -1_i64].contains(&value) {
            Err("invalid value".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl<'de> ::serde::Deserialize<'de> for BecStatus {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<i64>::deserialize(deserializer)?)
            .map_err(|e| <D::Error as ::serde::de::Error>::custom(e.to_string()))
    }
}
#[doc = "`Info`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"Info\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/$defs/ServiceInfo\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": true"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Info {
    ServiceInfo(ServiceInfoMessagePack),
    Object(::serde_json::Map<::std::string::String, ::serde_json::Value>),
}
impl ::std::convert::From<&Self> for Info {
    fn from(value: &Info) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<ServiceInfoMessagePack> for Info {
    fn from(value: ServiceInfoMessagePack) -> Self {
        Self::ServiceInfo(value)
    }
}
impl ::std::convert::From<::serde_json::Map<::std::string::String, ::serde_json::Value>> for Info {
    fn from(value: ::serde_json::Map<::std::string::String, ::serde_json::Value>) -> Self {
        Self::Object(value)
    }
}
#[doc = "`ServiceInfo`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"ServiceInfo\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"hostname\","]
#[doc = "    \"user\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"hostname\": {"]
#[doc = "      \"title\": \"Hostname\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"timestamp\": {"]
#[doc = "      \"title\": \"Timestamp\","]
#[doc = "      \"type\": \"number\""]
#[doc = "    },"]
#[doc = "    \"user\": {"]
#[doc = "      \"title\": \"User\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"versions\": {"]
#[doc = "      \"$ref\": \"#/$defs/ServiceVersions\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct ServiceInfo {
    pub hostname: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub timestamp: ::std::option::Option<f64>,
    pub user: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub versions: ::std::option::Option<ServiceVersions>,
}
impl ::std::convert::From<&ServiceInfo> for ServiceInfo {
    fn from(value: &ServiceInfo) -> Self {
        value.clone()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde:: Serialize, Clone)]
pub struct ServiceInfoInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: ServiceInfo,
}

#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct ServiceInfoMessagePack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: ServiceInfoInternal,
}

#[doc = "`ServiceVersions`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"ServiceVersions\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"bec_ipython_client\","]
#[doc = "    \"bec_lib\","]
#[doc = "    \"bec_server\","]
#[doc = "    \"bec_widgets\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"bec_ipython_client\": {"]
#[doc = "      \"title\": \"Bec Ipython Client\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"bec_lib\": {"]
#[doc = "      \"title\": \"Bec Lib\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"bec_server\": {"]
#[doc = "      \"title\": \"Bec Server\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"bec_widgets\": {"]
#[doc = "      \"title\": \"Bec Widgets\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct ServiceVersions {
    pub bec_ipython_client: ::std::string::String,
    pub bec_lib: ::std::string::String,
    pub bec_server: ::std::string::String,
    pub bec_widgets: ::std::string::String,
}
impl ::std::convert::From<&ServiceVersions> for ServiceVersions {
    fn from(value: &ServiceVersions) -> Self {
        value.clone()
    }
}

#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub enum ServiceStatus {
    RUNNING = 2,
    BUSY = 1,
    IDLE = 0,
    ERROR = -1,
}
#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct ServiceStatusPackInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: ServiceStatus,
}

#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct ServiceStatusPack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: ServiceStatusPackInternal,
}
#[doc = "Status message\n\nArgs:\n    name (str): Name of the status.\n    status (BECStatus): Value of the BECStatus enum (RUNNING = 2,  BUSY = 1, IDLE = 0, ERROR = -1).\n    info (ServiceInfo | dict): Status info.\n    metadata (dict, optional): Additional metadata."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"StatusMessage\","]
#[doc = "  \"description\": \"Status message\\n\\nArgs:\\n    name (str): Name of the status.\\n    status (BECStatus): Value of the BECStatus enum (RUNNING = 2,  BUSY = 1, IDLE = 0, ERROR = -1).\\n    info (ServiceInfo | dict): Status info.\\n    metadata (dict, optional): Additional metadata.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"info\","]
#[doc = "    \"name\","]
#[doc = "    \"status\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"info\": {"]
#[doc = "      \"title\": \"Info\","]
#[doc = "      \"oneOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/$defs/ServiceInfo\""]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"object\","]
#[doc = "          \"additionalProperties\": true"]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"metadata\": {"]
#[doc = "      \"title\": \"Metadata\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": true"]
#[doc = "    },"]
#[doc = "    \"name\": {"]
#[doc = "      \"title\": \"Name\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"status\": {"]
#[doc = "      \"$ref\": \"#/$defs/BECStatus\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct StatusMessage {
    pub info: Info,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub name: ::std::string::String,
    pub status: ServiceStatusPack,
}
impl ::std::convert::From<&StatusMessage> for StatusMessage {
    fn from(value: &StatusMessage) -> Self {
        value.clone()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde:: Serialize, Clone)]
pub struct StatusMessagePackInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: StatusMessage,
}

#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct StatusMessagePack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: StatusMessagePackInternal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmp_serde::from_slice;
    static TEST_MESSAGE_PACKED: [u8; 376] = *b"\x81\xad__bec_codec__\x83\xacencoder_name\xaaBECMessage\xa9type_name\xadStatusMessage\xa4data\x84\xa8metadata\x80\xa4name\xacDeviceServer\xa6status\x81\xad__bec_codec__\x83\xacencoder_name\xa4Enum\xa9type_name\xa9BECStatus\xa4data\x02\xa4info\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xabServiceInfo\xa4data\x84\xa4user\xa5david\xa8hostname\xaasusuwatari\xa9timestamp\xcbA\xda`\x8b\xa6|]\x12\xa8versions\x84\xa7bec_lib\xa63.91.0\xaabec_server\xa63.91.0\xb2bec_ipython_client\xa63.91.0\xabbec_widgets\xa72.45.13";
    #[test]
    fn test_deser_status_mesage() {
        let deser: StatusMessagePack = from_slice(&TEST_MESSAGE_PACKED).expect("Failed");
        let msg = deser.bec_codec.data;
        dbg!(&msg);
        match msg.info {
            Info::Object(_) => panic!("should be the other option"),
            Info::ServiceInfo(svc_info) => {
                assert_eq!(svc_info.bec_codec.data.hostname, "susuwatari");
                assert_eq!(
                    svc_info.bec_codec.data.versions.unwrap().bec_server,
                    "3.91.0"
                );
            }
        };
    }
}
