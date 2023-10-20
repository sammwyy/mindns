use std::io;

pub struct DualWriter {
    writer1: Box<dyn io::Write + Send + 'static>,
    writer2: Box<dyn io::Write + Send + 'static>,
}

impl DualWriter {
    pub fn new(
        writer1: Box<dyn io::Write + Send + 'static>,
        writer2: Box<dyn io::Write + Send + 'static>,
    ) -> Self {
        DualWriter { writer1, writer2 }
    }
}

impl io::Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result1 = self.writer1.write(buf);
        let _ = self.writer2.write(buf);
        result1
    }

    fn flush(&mut self) -> io::Result<()> {
        let flush1 = self.writer1.flush();
        let _ = self.writer2.flush();
        flush1
    }
}

unsafe impl Send for DualWriter {}

impl Into<Box<dyn std::io::Write + Send>> for DualWriter {
    fn into(self) -> Box<dyn std::io::Write + Send> {
        Box::new(self)
    }
}
