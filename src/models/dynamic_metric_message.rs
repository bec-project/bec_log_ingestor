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
#[doc = "`BoolDynamicMetricValue`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"_BoolDynamicMetricValue\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"value\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"type_name\": {"]
#[doc = "      \"title\": \"Type Name\","]
#[doc = "      \"default\": \"bool\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"bool\""]
#[doc = "    },"]
#[doc = "    \"value\": {"]
#[doc = "      \"title\": \"Value\","]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct BoolDynamicMetricValue {
    #[serde(default = "defaults::bool_dynamic_metric_value_type_name")]
    pub type_name: ::std::string::String,
    pub value: bool,
}
impl ::std::convert::From<&BoolDynamicMetricValue> for BoolDynamicMetricValue {
    fn from(value: &BoolDynamicMetricValue) -> Self {
        value.clone()
    }
}

#[doc = "Message for propagating metrics to the log ingestor.\n\nArgs:\n    metrics (dict[str, str|int|float|bool]): Mapping of name to metric value"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"DynamicMetricMessage\","]
#[doc = "  \"description\": \"Message for propagating metrics to the log ingestor.\\n\\nArgs:\\n    metrics (dict[str, str|int|float|bool]): Mapping of name to metric value\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"metrics\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"metadata\": {"]
#[doc = "      \"title\": \"Metadata\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": true"]
#[doc = "    },"]
#[doc = "    \"metrics\": {"]
#[doc = "      \"title\": \"Metrics\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": {"]
#[doc = "        \"oneOf\": ["]
#[doc = "          {"]
#[doc = "            \"$ref\": \"#/$defs/_StrDynamicMetricValue\""]
#[doc = "          },"]
#[doc = "          {"]
#[doc = "            \"$ref\": \"#/$defs/_IntDynamicMetricValue\""]
#[doc = "          },"]
#[doc = "          {"]
#[doc = "            \"$ref\": \"#/$defs/_FloatDynamicMetricValue\""]
#[doc = "          },"]
#[doc = "          {"]
#[doc = "            \"$ref\": \"#/$defs/_BoolDynamicMetricValue\""]
#[doc = "          }"]
#[doc = "        ],"]
#[doc = "        \"discriminator\": {"]
#[doc = "          \"mapping\": {"]
#[doc = "            \"bool\": \"#/$defs/_BoolDynamicMetricValue\","]
#[doc = "            \"float\": \"#/$defs/_FloatDynamicMetricValue\","]
#[doc = "            \"int\": \"#/$defs/_IntDynamicMetricValue\","]
#[doc = "            \"str\": \"#/$defs/_StrDynamicMetricValue\""]
#[doc = "          },"]
#[doc = "          \"propertyName\": \"type_name\""]
#[doc = "        }"]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"timestamp\": {"]
#[doc = "      \"title\": \"Timestamp\","]
#[doc = "      \"type\": \"number\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct DynamicMetricMessage {
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub metrics:
        ::std::collections::HashMap<::std::string::String, DynamicMetricMessageMetricsValueMsgPack>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub timestamp: ::std::option::Option<f64>,
}
impl ::std::convert::From<&DynamicMetricMessage> for DynamicMetricMessage {
    fn from(value: &DynamicMetricMessage) -> Self {
        value.clone()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde:: Serialize, Clone)]
pub struct DynamicMetricMessageMetricsValueMsgPackInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: DynamicMetricMessageMetricsValue,
}

#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct DynamicMetricMessageMetricsValueMsgPack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: DynamicMetricMessageMetricsValueMsgPackInternal,
}

#[doc = "`DynamicMetricMessageMetricsValue`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/$defs/_StrDynamicMetricValue\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/$defs/_IntDynamicMetricValue\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/$defs/_FloatDynamicMetricValue\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"$ref\": \"#/$defs/_BoolDynamicMetricValue\""]
#[doc = "    }"]
#[doc = "  ],"]
#[doc = "  \"discriminator\": {"]
#[doc = "    \"mapping\": {"]
#[doc = "      \"bool\": \"#/$defs/_BoolDynamicMetricValue\","]
#[doc = "      \"float\": \"#/$defs/_FloatDynamicMetricValue\","]
#[doc = "      \"int\": \"#/$defs/_IntDynamicMetricValue\","]
#[doc = "      \"str\": \"#/$defs/_StrDynamicMetricValue\""]
#[doc = "    },"]
#[doc = "    \"propertyName\": \"type_name\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum DynamicMetricMessageMetricsValue {
    StrDynamicMetricValue(StrDynamicMetricValue),
    IntDynamicMetricValue(IntDynamicMetricValue),
    FloatDynamicMetricValue(FloatDynamicMetricValue),
    BoolDynamicMetricValue(BoolDynamicMetricValue),
}
impl ::std::convert::From<&Self> for DynamicMetricMessageMetricsValue {
    fn from(value: &DynamicMetricMessageMetricsValue) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<StrDynamicMetricValue> for DynamicMetricMessageMetricsValue {
    fn from(value: StrDynamicMetricValue) -> Self {
        Self::StrDynamicMetricValue(value)
    }
}
impl ::std::convert::From<IntDynamicMetricValue> for DynamicMetricMessageMetricsValue {
    fn from(value: IntDynamicMetricValue) -> Self {
        Self::IntDynamicMetricValue(value)
    }
}
impl ::std::convert::From<FloatDynamicMetricValue> for DynamicMetricMessageMetricsValue {
    fn from(value: FloatDynamicMetricValue) -> Self {
        Self::FloatDynamicMetricValue(value)
    }
}
impl ::std::convert::From<BoolDynamicMetricValue> for DynamicMetricMessageMetricsValue {
    fn from(value: BoolDynamicMetricValue) -> Self {
        Self::BoolDynamicMetricValue(value)
    }
}
#[doc = "`FloatDynamicMetricValue`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"_FloatDynamicMetricValue\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"value\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"type_name\": {"]
#[doc = "      \"title\": \"Type Name\","]
#[doc = "      \"default\": \"float\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"float\""]
#[doc = "    },"]
#[doc = "    \"value\": {"]
#[doc = "      \"title\": \"Value\","]
#[doc = "      \"type\": \"number\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct FloatDynamicMetricValue {
    #[serde(default = "defaults::float_dynamic_metric_value_type_name")]
    pub type_name: ::std::string::String,
    pub value: f64,
}
impl ::std::convert::From<&FloatDynamicMetricValue> for FloatDynamicMetricValue {
    fn from(value: &FloatDynamicMetricValue) -> Self {
        value.clone()
    }
}
#[doc = "`IntDynamicMetricValue`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"_IntDynamicMetricValue\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"value\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"type_name\": {"]
#[doc = "      \"title\": \"Type Name\","]
#[doc = "      \"default\": \"int\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"int\""]
#[doc = "    },"]
#[doc = "    \"value\": {"]
#[doc = "      \"title\": \"Value\","]
#[doc = "      \"type\": \"integer\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct IntDynamicMetricValue {
    #[serde(default = "defaults::int_dynamic_metric_value_type_name")]
    pub type_name: ::std::string::String,
    pub value: i64,
}
impl ::std::convert::From<&IntDynamicMetricValue> for IntDynamicMetricValue {
    fn from(value: &IntDynamicMetricValue) -> Self {
        value.clone()
    }
}
#[doc = "`StrDynamicMetricValue`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"_StrDynamicMetricValue\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"value\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"type_name\": {"]
#[doc = "      \"title\": \"Type Name\","]
#[doc = "      \"default\": \"str\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"str\""]
#[doc = "    },"]
#[doc = "    \"value\": {"]
#[doc = "      \"title\": \"Value\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, PartialEq)]
pub struct StrDynamicMetricValue {
    #[serde(default = "defaults::str_dynamic_metric_value_type_name")]
    pub type_name: ::std::string::String,
    pub value: ::std::string::String,
}
impl ::std::convert::From<&StrDynamicMetricValue> for StrDynamicMetricValue {
    fn from(value: &StrDynamicMetricValue) -> Self {
        value.clone()
    }
}
#[doc = r" Generation of default values for serde."]
pub mod defaults {
    pub(super) fn bool_dynamic_metric_value_type_name() -> ::std::string::String {
        "bool".to_string()
    }
    pub(super) fn float_dynamic_metric_value_type_name() -> ::std::string::String {
        "float".to_string()
    }
    pub(super) fn int_dynamic_metric_value_type_name() -> ::std::string::String {
        "int".to_string()
    }
    pub(super) fn str_dynamic_metric_value_type_name() -> ::std::string::String {
        "str".to_string()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde:: Serialize, Clone)]
pub struct DynamicMetricPackInternal {
    pub encoder_name: String,
    pub type_name: String,
    pub data: DynamicMetricMessage,
}

#[derive(Debug, PartialEq, serde:: Deserialize, serde::Serialize, Clone)]
pub struct DynamicMetricMessagePack {
    #[serde(rename = "__bec_codec__")]
    pub bec_codec: DynamicMetricPackInternal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmp_serde::from_slice;

    static TEST_MESSAGE_PACKED: [u8; 227] = *b"\x81\xad__bec_codec__\x83\xacencoder_name\xaaBECMessage\xa9type_name\xb4DynamicMetricMessage\xa4data\x83\xa8metadata\x80\xa7metrics\x81\xabnum_devices\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb6_IntDynamicMetricValue\xa4data\x82\xa5value\xcc\xd6\xa9type_name\xa3int\xa9timestamp\xcbA\xdal\xa38\x9e\x12l";
    #[test]
    fn test_deser_dynamic_message_int() {
        let deser: DynamicMetricMessagePack = from_slice(&TEST_MESSAGE_PACKED).expect("Failed");
        let msg = deser.bec_codec.data;
        dbg!(&msg);
        let msg_value = msg
            .metrics
            .get("num_devices")
            .map(|i| i.bec_codec.data.clone());
        assert_eq!(
            msg_value,
            Some(DynamicMetricMessageMetricsValue::IntDynamicMetricValue(
                IntDynamicMetricValue {
                    type_name: "int".into(),
                    value: 214
                }
            ))
        )
    }

    static TEST_MULTI_MESSAGE_PACKED: [u8; 539] = *b"\x81\xad__bec_codec__\x83\xacencoder_name\xaaBECMessage\xa9type_name\xb4DynamicMetricMessage\xa4data\x83\xa8metadata\x80\xa7metrics\x84\xa1a\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb6_StrDynamicMetricValue\xa4data\x82\xa5value\xa6test_a\xa9type_name\xa3str\xa1b\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb6_IntDynamicMetricValue\xa4data\x82\xa5value\x05\xa9type_name\xa3int\xa1c\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb8_FloatDynamicMetricValue\xa4data\x82\xa5value\xcb@#\xcc\xcc\xcc\xcc\xcc\xcd\xa9type_name\xa5float\xa1d\x81\xad__bec_codec__\x83\xacencoder_name\xa9BaseModel\xa9type_name\xb7_BoolDynamicMetricValue\xa4data\x82\xa5value\xc2\xa9type_name\xa4bool\xa9timestamp\xcbA\xdal\xa4\x90\x93\x8f\xca";
    #[test]
    fn test_deser_dynamic_message_multi() {
        let deser: DynamicMetricMessagePack =
            from_slice(&TEST_MULTI_MESSAGE_PACKED).expect("Failed");
        let msg = deser.bec_codec.data;
        dbg!(&msg);

        let msg_value = msg.metrics.get("a").map(|i| i.bec_codec.data.clone());
        assert_eq!(
            msg_value,
            Some(DynamicMetricMessageMetricsValue::StrDynamicMetricValue(
                StrDynamicMetricValue {
                    type_name: "str".into(),
                    value: "test_a".into()
                }
            ))
        );

        let msg_value = msg.metrics.get("b").map(|i| i.bec_codec.data.clone());
        assert_eq!(
            msg_value,
            Some(DynamicMetricMessageMetricsValue::IntDynamicMetricValue(
                IntDynamicMetricValue {
                    type_name: "int".into(),
                    value: 5
                }
            ))
        );

        let msg_value = msg.metrics.get("c").map(|i| i.bec_codec.data.clone());
        assert_eq!(
            msg_value,
            Some(DynamicMetricMessageMetricsValue::FloatDynamicMetricValue(
                FloatDynamicMetricValue {
                    type_name: "float".into(),
                    value: 9.9
                }
            ))
        );

        let msg_value = msg.metrics.get("d").map(|i| i.bec_codec.data.clone());
        assert_eq!(
            msg_value,
            Some(DynamicMetricMessageMetricsValue::BoolDynamicMetricValue(
                BoolDynamicMetricValue {
                    type_name: "bool".into(),
                    value: false
                }
            ))
        );
    }
}
