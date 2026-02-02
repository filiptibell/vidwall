/*!
    Pipeline control signals.
*/

/**
    Signals for pipeline control.

    These are used to communicate state changes through the pipeline,
    such as end of stream or discontinuities from seeking.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PipelineSignal {
    /**
        Flush buffers — a discontinuity in the stream (e.g., after seeking).

        Recipients should clear any buffered data and reset internal state.
    */
    Flush,
    /**
        End of stream — no more data will be produced.

        Recipients should process any remaining buffered data and finalize.
    */
    Eos,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_equality() {
        assert_eq!(PipelineSignal::Flush, PipelineSignal::Flush);
        assert_eq!(PipelineSignal::Eos, PipelineSignal::Eos);
        assert_ne!(PipelineSignal::Flush, PipelineSignal::Eos);
    }

    #[test]
    fn signal_is_copy() {
        let s = PipelineSignal::Flush;
        let s2 = s; // Copy
        assert_eq!(s, s2);
    }
}
