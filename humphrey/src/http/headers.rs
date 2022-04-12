//! Provides functionality for handling HTTP headers.

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Headers(Vec<Header>);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Header {
    pub name: HeaderType,
    pub value: String,
}

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn add(&mut self, name: impl HeaderLike, value: impl AsRef<str>) {
        self.0.push(Header::new(name.as_header(), value));
    }

    pub fn push(&mut self, header: Header) {
        self.0.push(header);
    }

    pub fn get(&self, name: impl HeaderLike) -> Option<&str> {
        let header = name.as_header();
        self.0
            .iter()
            .find(|h| h.name == header)
            .map(|h| h.value.as_str())
    }

    pub fn get_mut(&mut self, name: impl HeaderLike) -> Option<&mut String> {
        let header = name.as_header();
        self.0
            .iter_mut()
            .find(|h| h.name == header)
            .map(|h| &mut h.value)
    }

    pub fn get_all(&self, name: impl HeaderLike) -> Vec<&str> {
        let header = name.as_header();
        self.0
            .iter()
            .filter(|h| h.name == header)
            .map(|h| h.value.as_str())
            .collect()
    }

    pub fn remove(&mut self, name: impl HeaderLike) {
        let header = name.as_header();
        self.0.retain(|h| h.name != header);
    }

    pub fn iter(&self) -> impl Iterator<Item = Header> {
        let mut headers = self.0.clone();
        headers.sort_unstable_by_key(|h| h.name.clone());
        headers.into_iter()
    }
}

impl Header {
    pub fn new(name: impl HeaderLike, value: impl AsRef<str>) -> Self {
        Self {
            name: name.as_header(),
            value: value.as_ref().to_string(),
        }
    }
}

pub trait HeaderLike {
    fn as_header(self) -> HeaderType;
}

impl HeaderLike for HeaderType {
    fn as_header(self) -> HeaderType {
        self
    }
}

impl HeaderLike for &HeaderType {
    fn as_header(self) -> HeaderType {
        self.clone()
    }
}

impl<T> HeaderLike for T
where
    T: AsRef<str>,
{
    fn as_header(self) -> HeaderType {
        HeaderType::from(self.as_ref())
    }
}

/// Represents a header received in a request.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HeaderType {
    /// Informs the server about the types of data that can be sent back.
    Accept,
    /// Informs the server about the accepted character encodings.
    AcceptCharset,
    /// Indicates the content encoding(s) understood by the client, usually compression algorithms.
    AcceptEncoding,
    /// Informs the server about the client's language(s).
    AcceptLanguage,
    /// Indicates the method that will be used for the actual request when performing an OPTIONS request.
    AccessControlRequestMethod,
    /// Provides credentials for HTTP authentication.
    Authorization,
    /// Indicates how the cache should behave.
    CacheControl,
    /// Indicates what should happen to the connection after the request is served.
    Connection,
    /// Lists any encodings used on the payload.
    ContentEncoding,
    /// Indicates the length of the payload body.
    ContentLength,
    /// Indicates the MIME type of the payload body.
    ContentType,
    /// Shares any applicable HTTP cookies with the server.
    Cookie,
    /// Indicates the date and time at which the request was sent.
    Date,
    /// Indicates any expectations that must be met by the server in order to properly serve the request.
    Expect,
    /// May contain reverse proxy information, generally not used in favour of the `X-Forwarded-For` header.
    Forwarded,
    /// Indicates the email address of the client, often used by crawlers.
    From,
    /// Specifies the host to which the request is being sent, e.g. "www.example.com".
    Host,
    /// Indicates the origin that caused the request.
    Origin,
    /// Contains backwards-compatible caching information.
    Pragma,
    /// Indicates the absolute or partial address of the page making the request.
    Referer,
    /// Indicates that the connection is to be upgraded to a different protocol, e.g. WebSocket.
    Upgrade,
    /// Informs the server of basic browser and device information.
    UserAgent,
    /// Contains the addresses of proxies through which the request has been forwarded.
    Via,
    /// Contains information about possible problems with the request.
    Warning,

    /// Indicates whether the response can be shared with other origins.
    AccessControlAllowOrigin,
    /// Contains the time in seconds that the object has been cached.
    Age,
    /// The set of methods supported by the resource.
    Allow,
    /// Indicates whether the response is to be displayed as a webpage or downloaded directly.
    ContentDisposition,
    /// Informs the client of the language of the payload body.
    ContentLanguage,
    /// Indicates an alternative location for the returned data.
    ContentLocation,
    /// Identifies a specific version of a resource.
    ETag,
    /// Contains the date and time at which the response is considered expired.
    Expires,
    /// Indicates the date and time at which the response was last modified.
    LastModified,
    /// Provides a means for serialising links in the headers, equivalent to the HTML `<link>` element.
    Link,
    /// Indicates the location at which the resource can be found, used for redirects.
    Location,
    /// Contains information about the server which served the request.
    Server,
    /// Indicates that the client should set the specified cookies.
    SetCookie,
    /// Indicates the encoding used in the transfer of the payload body.
    TransferEncoding,

    /// Custom header with a lowercase name
    Custom(String),
}

impl PartialOrd for HeaderType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.category() != other.category() {
            self.category().partial_cmp(&other.category())
        } else {
            self.to_string().partial_cmp(&other.to_string())
        }
    }
}

impl Ord for HeaderType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<&str> for HeaderType {
    fn from(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "accept" => Self::Accept,
            "accept-charset" => Self::AcceptCharset,
            "accept-encoding" => Self::AcceptEncoding,
            "accept-language" => Self::AcceptLanguage,
            "access-control-request-method" => Self::AccessControlRequestMethod,
            "authorization" => Self::Authorization,
            "cache-control" => Self::CacheControl,
            "connection" => Self::Connection,
            "content-encoding" => Self::ContentEncoding,
            "content-length" => Self::ContentLength,
            "content-type" => Self::ContentType,
            "cookie" => Self::Cookie,
            "date" => Self::Date,
            "expect" => Self::Expect,
            "forwarded" => Self::Forwarded,
            "from" => Self::From,
            "host" => Self::Host,
            "origin" => Self::Origin,
            "pragma" => Self::Pragma,
            "referer" => Self::Referer,
            "upgrade" => Self::Upgrade,
            "user-agent" => Self::UserAgent,
            "via" => Self::Via,
            "warning" => Self::Warning,
            "access-control-allow-origin" => Self::AccessControlAllowOrigin,
            "age" => Self::Age,
            "allow" => Self::Allow,
            "content-disposition" => Self::ContentDisposition,
            "content-language" => Self::ContentLanguage,
            "content-location" => Self::ContentLocation,
            "etag" => Self::ETag,
            "expires" => Self::Expires,
            "last-modified" => Self::LastModified,
            "link" => Self::Link,
            "location" => Self::Location,
            "server" => Self::Server,
            "set-cookie" => Self::SetCookie,
            "transfer-encoding" => Self::TransferEncoding,
            custom => Self::Custom(custom.to_string()),
        }
    }
}

impl ToString for HeaderType {
    fn to_string(&self) -> String {
        if let HeaderType::Custom(name) = self {
            return name.clone();
        }

        match self {
            HeaderType::Accept => "Accept",
            HeaderType::AcceptCharset => "Accept-Charset",
            HeaderType::AcceptEncoding => "Accept-Encoding",
            HeaderType::AcceptLanguage => "Accept-Language",
            HeaderType::AccessControlRequestMethod => "Access-Control-Request-Method",
            HeaderType::Authorization => "Authorization",
            HeaderType::CacheControl => "Cache-Control",
            HeaderType::Connection => "Connection",
            HeaderType::ContentEncoding => "Content-Encoding",
            HeaderType::ContentLength => "Content-Length",
            HeaderType::ContentType => "Content-Type",
            HeaderType::Cookie => "Cookie",
            HeaderType::Date => "Date",
            HeaderType::Expect => "Expect",
            HeaderType::Forwarded => "Forwarded",
            HeaderType::From => "From",
            HeaderType::Host => "Host",
            HeaderType::Origin => "Origin",
            HeaderType::Pragma => "Pragma",
            HeaderType::Referer => "Referer",
            HeaderType::Upgrade => "Upgrade",
            HeaderType::UserAgent => "User-Agent",
            HeaderType::Via => "Via",
            HeaderType::Warning => "Warning",
            HeaderType::AccessControlAllowOrigin => "Access-Control-Allow-Origin",
            HeaderType::Age => "Age",
            HeaderType::Allow => "Allow",
            HeaderType::ContentDisposition => "Content-Disposition",
            HeaderType::ContentLanguage => "Content-Language",
            HeaderType::ContentLocation => "Content-Location",
            HeaderType::ETag => "ETag",
            HeaderType::Expires => "Expires",
            HeaderType::LastModified => "Last-Modified",
            HeaderType::Link => "Link",
            HeaderType::Location => "Location",
            HeaderType::Server => "Server",
            HeaderType::SetCookie => "Set-Cookie",
            HeaderType::TransferEncoding => "Transfer-Encoding",
            _ => "",
        }
        .to_string()
    }
}

/// Represents a category of headers, as defined in [RFC 2616, section 4.2](https://datatracker.ietf.org/doc/html/rfc2616#section-4.2).
/// Used for ordering headers in responses.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum HeaderCategory {
    General,
    Response,
    Entity,
    Other,
}

impl HeaderType {
    fn category(&self) -> HeaderCategory {
        match self {
            HeaderType::AccessControlAllowOrigin => HeaderCategory::Other,
            HeaderType::Age => HeaderCategory::Response,
            HeaderType::Allow => HeaderCategory::Entity,
            HeaderType::CacheControl => HeaderCategory::General,
            HeaderType::Connection => HeaderCategory::General,
            HeaderType::ContentDisposition => HeaderCategory::Entity,
            HeaderType::ContentEncoding => HeaderCategory::Entity,
            HeaderType::ContentLanguage => HeaderCategory::Entity,
            HeaderType::ContentLength => HeaderCategory::Entity,
            HeaderType::ContentLocation => HeaderCategory::Entity,
            HeaderType::ContentType => HeaderCategory::Entity,
            HeaderType::Date => HeaderCategory::General,
            HeaderType::ETag => HeaderCategory::Response,
            HeaderType::Expires => HeaderCategory::Entity,
            HeaderType::LastModified => HeaderCategory::Entity,
            HeaderType::Link => HeaderCategory::Other,
            HeaderType::Location => HeaderCategory::Response,
            HeaderType::Pragma => HeaderCategory::General,
            HeaderType::Server => HeaderCategory::Response,
            HeaderType::SetCookie => HeaderCategory::Other,
            HeaderType::TransferEncoding => HeaderCategory::Entity,
            HeaderType::Upgrade => HeaderCategory::General,
            HeaderType::Via => HeaderCategory::General,
            HeaderType::Warning => HeaderCategory::General,
            HeaderType::Custom(_) => HeaderCategory::Other,
            HeaderType::Accept => HeaderCategory::Entity,
            HeaderType::AcceptCharset => HeaderCategory::Entity,
            HeaderType::AcceptEncoding => HeaderCategory::Entity,
            HeaderType::AcceptLanguage => HeaderCategory::Entity,
            HeaderType::AccessControlRequestMethod => HeaderCategory::Other,
            HeaderType::Authorization => HeaderCategory::General,
            HeaderType::Cookie => HeaderCategory::General,
            HeaderType::Expect => HeaderCategory::Entity,
            HeaderType::Forwarded => HeaderCategory::Response,
            HeaderType::From => HeaderCategory::Response,
            HeaderType::Host => HeaderCategory::General,
            HeaderType::Origin => HeaderCategory::General,
            HeaderType::Referer => HeaderCategory::General,
            HeaderType::UserAgent => HeaderCategory::General,
        }
    }
}
