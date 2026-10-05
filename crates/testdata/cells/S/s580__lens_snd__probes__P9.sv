// P9: S3 sign rule on the lower_expr_ctx path (continuous assign / fill-bearing sibling), signed patterns of another width
module t;
  logic signed [7:0] s8;
  logic signed [3:0] s4;
  logic [7:0] u8;
  wire c1, c2, c3, c5;
  wire [7:0] c4;
`ifndef NO_INSIDE
  assign c1 = s8 inside {4'sb?100};            // -4: 1
  assign c2 = s4 inside {8'sb11111?00};        // -4: 1
  assign c3 = u8 inside {4'sb?100};            // 11110100 unsigned: 0
  assign c4 = (s4 inside {8'sb11111?00}) + '1; // 1 + 8'hFF = 8'h00
  assign c5 = '1 inside {s4 + 4'sb0000, 8'sb11111?11}; // '1 sized by the context; 8-bit all-ones vs 11111?11: 1
`endif
  wire q1 = s8 ==? 4'sb?100, q2 = s4 ==? 8'sb11111?00, q3 = u8 ==? 4'sb?100;
  wire [7:0] q4 = (s4 ==? 8'sb11111?00) + '1;
  initial begin
    s8 = -8'sd4; s4 = -4'sd4; u8 = 8'b1111_0100;
    #1;
`ifndef NO_INSIDE
    $display("c1=%b c2=%b c3=%b c4=%h c5=%b", c1, c2, c3, c4, c5);
`endif
    $display("q1=%b q2=%b q3=%b q4=%h", q1, q2, q3, q4);
    #1 $finish;
  end
endmodule
