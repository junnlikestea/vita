use addr::DomainName;
use std::collections::HashSet;
use std::hash::Hash;

/// Represents the filtering applied to the output
enum Filter {
    /// Return any result that matches the same subdomain
    SubOnly,
    /// Return any result that has the same root domain
    RootOnly,
}

impl Default for Filter {
    fn default() -> Self {
        Self::RootOnly
    }
}

/// `PostProcessor` is responsible for filtering the raw data from each of the data sources into
/// only those results which are relevant.
#[derive(Default)]
pub struct PostProcessor {
    roots: HashSet<String>,
    filter: Filter,
}

impl PostProcessor {
    /// Sets the `PostProcessor` to return any result which matches the same root domain
    pub fn any_root<I: IntoIterator<Item = String>>(&mut self, hosts: I) -> &mut Self {
        self.roots = hosts
            .into_iter()
            .filter_map(|d| d.parse::<DomainName>().ok())
            .map(|d| d.root().to_string())
            .collect();
        self.filter = Filter::RootOnly;
        self
    }

    /// Sets the `PostProcessor` to return any result which matches the same subdomain
    pub fn any_subdomain<I: IntoIterator<Item = String>>(&mut self, hosts: I) -> &mut Self {
        self.roots.extend(hosts);
        self.filter = Filter::SubOnly;
        self
    }

    /// Strips invalid characters from the domain
    ///
    /// Used before attempting to parse a domain into  a `add::DomainName`.
    ///
    /// Errors
    /// If the the input domain contains any invalid characters the
    /// attempting to parse it into a `addr::DomainName` would return an error
    fn strip_invalid<T: Into<String>>(domain: T) -> String {
        let blacklisted = vec!["\"", "\\", "*"];
        // iter over the blacklisted chars and return a string that has been cleaned.
        blacklisted.iter().fold(domain.into(), |mut res, c| {
            res = res.replace(c, "");
            res.strip_prefix('.').unwrap_or(&res).to_lowercase()
        })
    }

    /// Determines if the domain is a result we're interested in
    ///
    /// By default, a "relevant" result is any result that has the same root domain
    /// as one of the input domains. For example, if you provided `example.com` as an
    /// input domain `some.sub.example.com` would be a relevant result using default
    /// config. The non-default `Filter::SubOnly` will apply no filtering to the input
    /// domains and a relevant result will be any that has the same suffix as one of the
    /// input domains.
    fn is_relevant<T: AsRef<str>>(&self, result: T) -> bool {
        let cleaned_result = Self::strip_invalid(result.as_ref());
        match self.filter {
            Filter::RootOnly => {
                if let Ok(d) = cleaned_result.parse::<DomainName>() {
                    self.roots.contains(d.root().to_str())
                } else {
                    false
                }
            }
            Filter::SubOnly => self
                .roots
                .iter()
                .any(|root| cleaned_result.ends_with(root) && !cleaned_result.eq(root)),
        }
    }
}

pub struct PostProcessorIter<'a, I>
where
    I: Iterator,
{
    cleaner: &'a PostProcessor,
    inner: I,
}

impl<'a, I> Iterator for PostProcessorIter<'a, I>
where
    I: Iterator,
    I::Item: Hash + Eq + AsRef<str>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(d) = self.inner.next() {
            if self.cleaner.is_relevant(d.as_ref()) {
                return Some(d);
            }
        }
        None
    }
}

pub trait CleanExt: Iterator {
    fn clean(self, postprocessor: &PostProcessor) -> PostProcessorIter<Self>
    where
        Self::Item: Hash + Eq + AsRef<str>,
        Self: Sized,
    {
        PostProcessorIter {
            cleaner: postprocessor,
            inner: self,
        }
    }
}

impl<I: Iterator> CleanExt for I {}
