`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic scl_o, scl_en, sda_o, sda_en, scl, sda; logic [14:0] intr; integer k; logic [63:0] sdig;
  assign scl = scl_en ? scl_o : 1'b1;
  assign sda = sda_en ? sda_o : 1'b1;
  i2c dut (.clk_i(clk), .rst_ni(rst_n), .ram_cfg_i('0), .ram_cfg_o(), .tl_i(h), .tl_o(d),
    .alert_rx_i(4'b0101), .alert_tx_o(), .racl_policies_i('0), .racl_error_o(),
    .cio_scl_i(scl), .cio_scl_o(scl_o), .cio_scl_en_o(scl_en), .cio_sda_i(sda), .cio_sda_o(sda_o), .cio_sda_en_o(sda_en),
    .lsio_trigger_o(),
    .intr_fmt_threshold_o(intr[0]), .intr_rx_threshold_o(intr[1]), .intr_acq_threshold_o(intr[2]), .intr_rx_overflow_o(intr[3]),
    .intr_controller_halt_o(intr[4]), .intr_scl_interference_o(intr[5]), .intr_sda_interference_o(intr[6]), .intr_stretch_timeout_o(intr[7]),
    .intr_sda_unstable_o(intr[8]), .intr_cmd_complete_o(intr[9]), .intr_tx_stretch_o(intr[10]), .intr_tx_threshold_o(intr[11]),
    .intr_acq_stretch_o(intr[12]), .intr_unexp_stop_o(intr[13]), .intr_host_timeout_o(intr[14]));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ 64'({scl_o, scl_en, sda_o, sda_en, intr});
  initial begin
    sdig = 64'h0;
    `TB_RESET
    for (k = 0; k <= 32'h7c; k = k + 4) `TLR(32'(k))
    `TLW(32'h04, 32'h7fff)        // intr enable
    `TLW(32'h3c, 32'h0004_0004)   // TIMING0 thigh/tlow
    `TLW(32'h40, 32'h0001_0001)
    `TLW(32'h44, 32'h0004_0004)
    `TLW(32'h48, 32'h0002_0002)
    `TLW(32'h4c, 32'h0004_0004)
    `TLW(32'h10, 32'h0000_0001)   // CTRL: host enable
    `TLW(32'h1c, 32'h0000_01a4)   // FDATA: start + addr byte
    `TLW(32'h1c, 32'h0000_005a)
    `TLW(32'h1c, 32'h0000_02c3)   // stop
    repeat (2000) @(posedge clk);
    for (k = 0; k <= 32'h7c; k = k + 4) `TLR(32'(k))
    $display("S sdig=%h intr=%h", sdig, intr);
    `TB_END
  end
  initial begin #3000000 $display("WATCHDOG"); $finish; end
endmodule
