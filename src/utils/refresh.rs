pub struct CachedMethod<T> {
    func: fn() -> T,
    cache: Option<T>,
}

impl<T> CachedMethod<T> {
    pub fn get(&mut self) -> &T {
        self.cache.get_or_insert_with(self.func)
    }
}

pub struct CachedSelfRefMethod<S, T> {
    func: fn(&S) -> T,
    cache: Option<T>,
}

impl<S, T> CachedSelfRefMethod<S, T> {
    pub fn get(&mut self, s: &S) -> &T {
        self.cache.get_or_insert_with(|| (self.func)(s))
    }
}
