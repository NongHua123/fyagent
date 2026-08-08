#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MsiStringProbeStatus {
    Success,
    MoreData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MsiStringProbeDisposition {
    Empty,
    RequiredLength(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct UnexpectedSuccessLength;

pub(crate) const fn classify_msi_string_probe(
    status: MsiStringProbeStatus,
    reported_length: u32,
) -> Result<MsiStringProbeDisposition, UnexpectedSuccessLength> {
    match (status, reported_length) {
        (MsiStringProbeStatus::Success, 0) | (MsiStringProbeStatus::MoreData, 0) => {
            Ok(MsiStringProbeDisposition::Empty)
        }
        (MsiStringProbeStatus::MoreData, length) => {
            Ok(MsiStringProbeDisposition::RequiredLength(length))
        }
        (MsiStringProbeStatus::Success, _) => Err(UnexpectedSuccessLength),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_both_documented_empty_probe_results() {
        assert_eq!(
            classify_msi_string_probe(MsiStringProbeStatus::Success, 0),
            Ok(MsiStringProbeDisposition::Empty)
        );
        assert_eq!(
            classify_msi_string_probe(MsiStringProbeStatus::MoreData, 0),
            Ok(MsiStringProbeDisposition::Empty)
        );
    }

    #[test]
    fn requests_a_second_read_only_for_positive_more_data_lengths() {
        for length in [1, 1_024, 32_767] {
            assert_eq!(
                classify_msi_string_probe(MsiStringProbeStatus::MoreData, length),
                Ok(MsiStringProbeDisposition::RequiredLength(length))
            );
        }
    }

    #[test]
    fn rejects_success_that_claims_unwritten_data() {
        assert_eq!(
            classify_msi_string_probe(MsiStringProbeStatus::Success, 1),
            Err(UnexpectedSuccessLength)
        );
    }
}
