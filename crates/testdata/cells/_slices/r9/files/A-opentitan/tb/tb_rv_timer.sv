`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic intr; integer k; logic [63:0] sdig;
  rv_timer dut (.clk_i(clk), .rst_ni(rst_n), .tl_i(h), .tl_o(d), .alert_rx_i(4'b0101), .alert_tx_o(),
    .racl_policies_i('0), .racl_error_o(), .intr_timer_expired_hart0_timer0_o(intr));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ 64'(intr);
  initial begin
    sdig = 64'h0;
    `TB_RESET
    `TLR(32'h0) `TLR(32'h4)
    for (k = 32'h100; k <= 32'h11c; k = k + 4) `TLR(32'(k))
    `TLW(32'h10c, 32'h0002_0003)  // cfg0: step 2, prescale 3
    `TLW(32'h118, 32'h0000_0040)  // compare lo
    `TLW(32'h11c, 32'h0000_0000)  // compare hi
    `TLW(32'h100, 32'h1)          // intr enable
    `TLW(32'h4, 32'h1)            // ctrl active
    repeat (300) @(posedge clk);
    for (k = 32'h100; k <= 32'h11c; k = k + 4) `TLR(32'(k))
    `TLW(32'h104, 32'h1) `TLR(32'h104)
    `TLW(32'h118, 32'hffff_fff0) `TLW(32'h11c, 32'hffff_ffff) `TLW(32'h104, 32'h1) `TLR(32'h104)
    `TLW(32'h108, 32'h1) `TLR(32'h104)
    `TLR(32'h110) `TLR(32'h114)
    $display("S sdig=%h intr=%b", sdig, intr);
    `TB_END
  end
  initial begin #3000000 $display("WATCHDOG"); $finish; end
endmodule
