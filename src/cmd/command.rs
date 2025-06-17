

pub enum RedisCommand {
    Connection(ConnCmd),
    String(StringCmd),
    List(ListCmd),
    Set(SetCmd),
    SortSet(SortedCmd),
    Hash(HashCmd),
}
pub enum ConnCmd {
    /// Authenticates the connection
    Auth,
    /// A container for client connection commands
    Client,
    /// Returns the given string
    Echo,
    /// Handshakes with the Redis server
    Hello,
    /// Returns the server's liveliness response
    Ping,
    /// Closes the connection
    Quit,
    /// Resets the connection
    Reset,
    /// Changes the selected database
    Select,
}

pub enum HashCmd {
    /// Deletes one or more fields and their values from a hash.
    HDel,
    /// Returns the value of a field in a hash
    HGet,
    /// Creates or modifies the value of a field in a hash
    HSet,
    /// Iterates over fields and values of a hash
    HScan,
}

pub enum ListCmd {
    /// Prepends one or more elements to a list. Creates the key if it doesn't exist
    LPush,
    /// Appends one or more elements to a list. Creates the key if it doesn't exist
    RPush,
    /// Returns the first elements in a list after removing it. Deletes the list if the last element was popped
    LPop,
    /// Returns and removes the last elements of a list. Deletes the list if the last element was popped
    RPop,
    /// Sets the value of an element in a list by its index
    LSet,
    /// Inserts an element before or after another element in a list
    LInsert,
    /// Returns the length of a list
    LLen,
}

pub enum SetCmd {
    /// Returns the number of members in a set
    SCard,
    /// Adds one or more members to a set
    SAdd,
    /// Iterates over members of a set
    SSCan,
    /// Returns the union of multiple sets
    SUnion,
    /// Returns the intersect of multiple sets
    SInter,
    /// Returns the number of members of the intersect of multiple sets
    SInterCard,
}

pub enum SortedCmd {
    /// Adds one or more members to a sorted set, or updates their scores.
    ZAdd,
    /// Returns the number of members in a sorted set
    ZCard,
    /// Returns the score of a member in a sorted set
    ZScore,
    /// Returns the union of multiple sorted sets
    ZUnion,
    /// Returns the intersect of multiple sorted sets
    ZInter,
    /// Returns the number of members of the intersect of multiple sorted sets
    ZInterCard,
    /// Stores the intersect of multiple sorted sets in a key
    ZInterStore,
}

pub enum StringCmd {
    /// Appends a string to the value of a key. Creates the key if it doesn't exist
    Append,
    /// Returns the string value of a key
    Get,
    /// Sets the string value of a key, ignoring its type. The key is created if it doesn't exist
    Set,
    /// Returns the length of a string value
    Strlen,
    /// Increments the integer value of a key by one
    Incr,
    /// Increments the integer value of a key by a number
    IncrBy,
    /// Decrements the integer value of a key by one
    Decr,
    /// Decrements a number from the integer value of a key
    DecrBy,
}