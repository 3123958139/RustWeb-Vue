//! # 公共帧提取器
//!
//! 从原始字节流中提取固定长度帧：缓存输入、搜索帧头、校验并解码。
//! 原 fj200c_information / ftj1c 各自一份同构实现，统一收拢到此公共模块，
//! 通过 `header` / `frame_size` / 回调闭包参数化适应不同协议。

/// 帧提取器：从连续字节流中定位和提取固定长度的帧
pub struct FrameExtractor {
    /// 帧头标识字节序列（如 `[0xEB, 0x90, 0x64]`）
    header: Vec<u8>,
    /// 每帧的固定长度（字节数）
    frame_size: usize,
    /// 帧校验闭包：接收完整帧，返回是否通过校验
    validator: Box<dyn Fn(&[u8]) -> bool + Send + Sync>,
    /// 帧解码闭包：接收校验通过的帧，执行解码逻辑
    decoder: Box<dyn Fn(&[u8]) -> bool + Send + Sync>,
    /// 数据缓冲区：累积尚未处理的输入字节
    buffer: Vec<u8>,
}

impl FrameExtractor {
    /// 创建帧提取器实例
    ///
    /// - `header`：帧头标识字节序列
    /// - `frame_size`：每帧固定长度
    /// - `validator`：帧校验闭包
    /// - `decoder`：帧解码闭包
    pub fn new(
        header: Vec<u8>,
        frame_size: usize,
        validator: Box<dyn Fn(&[u8]) -> bool + Send + Sync>,
        decoder: Box<dyn Fn(&[u8]) -> bool + Send + Sync>,
    ) -> Self {
        Self {
            header,
            frame_size,
            validator,
            decoder,
            // 预分配 2 帧大小的缓冲区容量，减少频繁扩容
            buffer: Vec::with_capacity(frame_size * 2),
        }
    }

    /// 注入数据并尝试提取帧
    ///
    /// 将新数据追加到缓冲区，然后循环扫描提取帧。
    /// 返回本次提取到的帧数量。
    pub fn feed(&mut self, data: &[u8]) -> usize {
        self.buffer.extend_from_slice(data);
        self.process()
    }

    /// 内部处理循环：扫描缓冲区，定位帧头、校验、解码
    fn process(&mut self) -> usize {
        let mut extracted = 0;

        loop {
            // 缓冲区数据不足一帧时退出
            if self.buffer.len() < self.frame_size {
                break;
            }

            // 在缓冲区中搜索帧头
            match find_subslice(&self.buffer, &self.header) {
                Some(pos) => {
                    // 丢弃帧头之前的数据（非帧数据或不完整的帧片段）
                    if pos > 0 {
                        self.buffer.drain(..pos);
                    }
                    // 帧头后数据不足一帧时退出
                    if self.buffer.len() < self.frame_size {
                        break;
                    }

                    // 截取完整帧数据进行校验
                    let frame = self.buffer[..self.frame_size].to_vec();
                    if (self.validator)(&frame) {
                        // 校验通过，调用解码器
                        let ok = (self.decoder)(&frame);
                        if ok {
                            extracted += 1;
                        }
                        // 从缓冲区移除已处理的帧
                        self.buffer.drain(..self.frame_size);
                    } else {
                        // 校验失败，丢弃帧头的第一个字节，继续搜索
                        self.buffer.drain(..1);
                    }
                }
                None => {
                    // 未找到帧头，清空缓冲区避免内存无限增长
                    self.buffer.clear();
                    break;
                }
            }
        }

        extracted
    }
}

/// 在数据中查找子序列首次出现的位置（朴素匹配算法）
///
/// 使用滑动窗口 `windows(len)` 遍历，适用于 3 字节帧头的快速定位。
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
