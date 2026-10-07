`include "tlhost.svh"
module tb;
  `TB_COMMON
  tlul_cmd_intg_gen u_gen (.tl_i(h_raw), .tl_o(h));
  logic [31:0] gin, gout, goe, intr; logic strap_en; integer k;
  logic [63:0] sdig;
  gpio dut (.clk_i(clk), .rst_ni(rst_n), .strap_en_i(strap_en), .sampled_straps_o(),
    .tl_i(h), .tl_o(d), .intr_gpio_o(intr), .alert_rx_i(4'b0101), .alert_tx_o(),
    .racl_policies_i('0), .racl_error_o(), .cio_gpio_i(gin), .cio_gpio_o(gout), .cio_gpio_en_o(goe));
  always @(posedge clk) if (!rst_n) sdig <= 64'h0; else sdig <= {sdig[62:0], sdig[63]} ^ {gout, goe} ^ 64'(intr);
  initial begin
    gin = 32'h0; strap_en = 1'b0; sdig = 64'h0;
    `TB_RESET
    @(negedge clk); gin = 32'hc3a5_0f01; strap_en = 1'b1; @(posedge clk); @(negedge clk); strap_en = 1'b0;
    for (k = 0; k <= 32'h44; k = k + 4) `TLR(32'(k))
    `TLW(32'h04, 32'hffff_ffff)  // intr enable
    `TLW(32'h2c, 32'h0000_ffff)  // rising
    `TLW(32'h30, 32'hffff_0000)  // falling
    `TLW(32'h14, 32'h1234_5678)  // direct out
    `TLW(32'h20, 32'h00ff_ff00)  // direct oe
    `TLW(32'h18, 32'h00f0_0a0a)  // masked out lower
    `TLW(32'h28, 32'hff00_00ff)  // masked oe upper
    repeat (10) @(posedge clk);
    @(negedge clk); gin = 32'h3c5a_f0f0; repeat (10) @(posedge clk);
    @(negedge clk); gin = 32'h0f0f_1234; repeat (10) @(posedge clk);
    `TLW(32'h3c, 32'h0000_00ff)  // input filter
    @(negedge clk); gin = 32'hffff_ffff; repeat (5) @(posedge clk); @(negedge clk); gin = 32'h0; repeat (30) @(posedge clk);
    for (k = 0; k <= 32'h44; k = k + 4) `TLR(32'(k))
    `TLW(32'h00, 32'hffff_ffff) `TLR(32'h00)
    `TLW(32'h08, 32'h0000_00a5) `TLR(32'h00)
    $display("S sdig=%h intr=%h gout=%h goe=%h", sdig, intr, gout, goe);
    `TB_END
  end
  initial begin #2000000 $display("WATCHDOG"); $finish; end
endmodule
