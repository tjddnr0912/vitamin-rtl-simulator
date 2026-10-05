module top;
  localparam logic signed [63:0] S64N = -64'sd4;
  if (S64N inside {4'sb1?00}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
