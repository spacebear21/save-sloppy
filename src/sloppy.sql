CREATE TABLE IF NOT EXISTS transaction (
  id               INTEGER PRIMARY KEY,
  type             TEXT CHECK(type IN ('incoming', 'outgoing'))
  invoice          TEXT
  description      TEXT
  description_hash TEXT
  preimage         TEXT
  payment_hash     TEXT NOT NULL
  amount_high      INTEGER NOT NULL
  amount_low       INTEGER NOT NULL
  fees_paid_high   INTEGER NOT NULL
  fees_paid_low    INTEGER NOT NULL
  created_at       TEXT NOT NULL
  expires_at       TEXT
  settled_at       TEXT
  metadata         TEXT
)

INSERT INTO transaction

  /**
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<TransactionType>,
    /// Bolt11 invoice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    /// Invoice's description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Invoice's description hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
    /// Payment preimage
    #[serde(skip_serializing_if = "Option::is_none")]
   pub preimage: Option<String>,
    /// Payment hash
    pub payment_hash: String,
    /// Amount in millisatoshis
    pub amount: u64,
    /// Fees paid in millisatoshis
    pub fees_paid: u64,
    /// Creation timestamp in seconds since epoch
    pub created_at: Timestamp,
    /// Expiration timestamp in seconds since epoch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
    /// Settled timestamp in seconds since epoch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<Timestamp>,
    /// Optional metadata about the payment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
**/
