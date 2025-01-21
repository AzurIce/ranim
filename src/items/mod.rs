pub mod vitem;

pub trait Extract {
    type ExtractData;

    fn extract(&self) -> Self::ExtractData;
}

