`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic [7:0] po; logic [1:0] intr; integer k; logic [63:0] sdig;
  pattgen dut (.clk_i(clk), .rst_ni(rst_n), .tl_i(h), .tl_o(d), .alert_rx_i(4'b0101), .alert_tx_o(),
    .cio_pda0_tx_o(po[0]), .cio_pcl0_tx_o(po[1]), .cio_pda1_tx_o(po[2]), .cio_pcl1_tx_o(po[3]),
    .cio_pda0_tx_en_o(po[4]), .cio_pcl0_tx_en_o(po[5]), .cio_pda1_tx_en_o(po[6]), .cio_pcl1_tx_en_o(po[7]),
    .intr_done_ch0_o(intr[0]), .intr_done_ch1_o(intr[1]));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ 64'({po, intr});
  initial begin
    sdig = 64'h0;
    `TB_RESET
    `TB_SWEEP(32'h3c)
    `TLW(32'h10, 32'h0) // ctrl off
    `TLW(32'h04, 32'h3) // intr enable
    `TLW(32'h14, 32'h1) `TLW(32'h18, 32'h2)          // prediv
    `TLW(32'h1c, 32'ha5c3_0f96) `TLW(32'h20, 32'h1234_5678) `TLW(32'h24, 32'hdead_beef) `TLW(32'h28, 32'h0bad_cafe)
    `TLW(32'h2c, 32'h0003_0207) // size: len/reps
    `TLW(32'h10, 32'h0000_0003) // enable both
    repeat (1500) @(posedge clk);
    for (k = 0; k <= 32'h3c; k = k + 4) `TLR(32'(k))
    $display("S sdig=%h po=%b intr=%b", sdig, po, intr);
    `TB_END
  end
  initial begin #2000000 $display("WATCHDOG"); $finish; end
endmodule
