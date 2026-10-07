`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic clk_aon; initial clk_aon = 1'b0; always #23 clk_aon = ~clk_aon;
  logic [4:0] o; integer k; logic [63:0] sdig;
  aon_timer dut (.clk_i(clk), .clk_aon_i(clk_aon), .rst_ni(rst_n), .rst_aon_ni(rst_n), .tl_i(h), .tl_o(d),
    .alert_rx_i(4'b0101), .alert_tx_o(), .racl_policies_i('0), .racl_error_o(), .lc_escalate_en_i(4'b1010),
    .intr_wkup_timer_expired_o(o[0]), .intr_wdog_timer_bark_o(o[1]), .nmi_wdog_timer_bark_o(o[2]), .wkup_req_o(o[3]),
    .aon_timer_rst_req_o(o[4]), .sleep_mode_i(1'b0));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ 64'(o);
  initial begin
    sdig = 64'h0;
    `TB_RESET
    for (k = 0; k <= 32'h34; k = k + 4) `TLR(32'(k))
    `TLW(32'h0c, 32'h0000_0020)   // wkup thold lo
    `TLW(32'h08, 32'h0000_0000)
    `TLW(32'h04, 32'h0000_0001)   // wkup enable, prescale 0
    `TLW(32'h20, 32'h0000_0030)   // bark
    `TLW(32'h24, 32'h0000_0050)   // bite
    `TLW(32'h1c, 32'h0000_0001)   // wdog enable
    repeat (3000) @(posedge clk);
    for (k = 0; k <= 32'h34; k = k + 4) `TLR(32'(k))
    `TLW(32'h2c, 32'h3) repeat (100) @(posedge clk); `TLR(32'h2c)
    $display("S sdig=%h o=%b", sdig, o);
    `TB_END
  end
  initial begin #3000000 $display("WATCHDOG"); $finish; end
endmodule
