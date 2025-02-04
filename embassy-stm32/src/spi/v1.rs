#![macro_use]

pub use embedded_hal::blocking;
pub use embedded_hal::spi::{Mode, Phase, Polarity, MODE_0, MODE_1, MODE_2, MODE_3};
use futures::future::join;

use super::*;

impl<'d, T: Instance, Tx, Rx> Spi<'d, T, Tx, Rx> {
    pub(super) async fn write_dma_u8(&mut self, write: &[u8]) -> Result<(), Error>
    where
        Tx: TxDmaChannel<T>,
    {
        unsafe {
            T::regs().cr1().modify(|w| {
                w.set_spe(false);
            });
        }
        self.set_word_size(WordSize::EightBit);

        let request = self.txdma.request();
        let dst = T::regs().tx_ptr();
        let f = crate::dma::write(&mut self.txdma, request, write, dst);

        unsafe {
            T::regs().cr2().modify(|reg| {
                reg.set_txdmaen(true);
            });
            T::regs().cr1().modify(|w| {
                w.set_spe(true);
            });
        }

        f.await;

        finish_dma(T::regs());

        Ok(())
    }

    pub(super) async fn read_dma_u8(&mut self, read: &mut [u8]) -> Result<(), Error>
    where
        Tx: TxDmaChannel<T>,
        Rx: RxDmaChannel<T>,
    {
        unsafe {
            T::regs().cr1().modify(|w| {
                w.set_spe(false);
            });
            T::regs().cr2().modify(|reg| {
                reg.set_rxdmaen(true);
            });
        }
        self.set_word_size(WordSize::EightBit);

        let clock_byte_count = read.len();

        let rx_request = self.rxdma.request();
        let rx_src = T::regs().rx_ptr();
        let rx_f = crate::dma::read(&mut self.rxdma, rx_request, rx_src, read);

        let tx_request = self.txdma.request();
        let tx_dst = T::regs().tx_ptr();
        let clock_byte = 0x00u8;
        let tx_f = crate::dma::write_repeated(
            &mut self.txdma,
            tx_request,
            clock_byte,
            clock_byte_count,
            tx_dst,
        );

        unsafe {
            T::regs().cr2().modify(|reg| {
                reg.set_txdmaen(true);
            });
            T::regs().cr1().modify(|w| {
                w.set_spe(true);
            });
        }

        join(tx_f, rx_f).await;

        finish_dma(T::regs());

        Ok(())
    }

    pub(super) async fn read_write_dma_u8(
        &mut self,
        read: &mut [u8],
        write: &[u8],
    ) -> Result<(), Error>
    where
        Tx: TxDmaChannel<T>,
        Rx: RxDmaChannel<T>,
    {
        assert!(read.len() >= write.len());

        unsafe {
            T::regs().cr1().modify(|w| {
                w.set_spe(false);
            });
            T::regs().cr2().modify(|reg| {
                reg.set_rxdmaen(true);
            });
        }
        self.set_word_size(WordSize::EightBit);

        let rx_request = self.rxdma.request();
        let rx_src = T::regs().rx_ptr();
        let rx_f = crate::dma::read(
            &mut self.rxdma,
            rx_request,
            rx_src,
            &mut read[0..write.len()],
        );

        let tx_request = self.txdma.request();
        let tx_dst = T::regs().tx_ptr();
        let tx_f = crate::dma::write(&mut self.txdma, tx_request, write, tx_dst);

        unsafe {
            T::regs().cr2().modify(|reg| {
                reg.set_txdmaen(true);
            });
            T::regs().cr1().modify(|w| {
                w.set_spe(true);
            });
        }

        join(tx_f, rx_f).await;

        finish_dma(T::regs());

        Ok(())
    }
}
