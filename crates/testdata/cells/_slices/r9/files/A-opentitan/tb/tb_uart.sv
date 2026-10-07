`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic tx, tx_en; logic [8:0] intr;
  uart dut (.clk_i(clk), .rst_ni(rst_n), .tl_i(h), .tl_o(d),
    .alert_rx_i(4'b0101), .alert_tx_o(), .racl_policies_i('0), .racl_error_o(), .lsio_trigger_o(),
    .cio_rx_i(tx), .cio_tx_o(tx), .cio_tx_en_o(tx_en),
    .intr_tx_watermark_o(intr[0]), .intr_tx_empty_o(intr[1]), .intr_rx_watermark_o(intr[2]), .intr_tx_done_o(intr[3]),
    .intr_rx_overflow_o(intr[4]), .intr_rx_frame_err_o(intr[5]), .intr_rx_break_err_o(intr[6]), .intr_rx_timeout_o(intr[7]),
    .intr_rx_parity_err_o(intr[8]));
  integer k;
  initial begin
    `TB_RESET
    for (k = 0; k < 13; k = k + 1) `TLR(32'(k * 4))
    `TLW(32'h04, 32'h1ff)          // INTR_ENABLE all
    `TLW(32'h10, 32'h8000_0063)    // CTRL: NCO=0x8000, TX, RX, parity en+odd
    `TLW(32'h1c, 32'h5a) `TLW(32'h1c, 32'ha5) `TLW(32'h1c, 32'h3c)
    `TLR(32'h14) `TLR(32'h24)
    repeat (1500) @(posedge clk);
    $display("I intr=%b tx=%b", intr, tx);
    `TLR(32'h14) `TLR(32'h24) `TLR(32'h18) `TLR(32'h18) `TLR(32'h18) `TLR(32'h14) `TLR(32'h00)
    `TLW(32'h00, 32'h1ff) `TLR(32'h00)
    `TLW(32'h08, 32'h0aa) `TLR(32'h00)
    `TLR(32'h40)                    // unmapped -> error
    `TB_END
  end
  initial begin #2000000 $display("WATCHDOG"); $finish; end
endmodule
