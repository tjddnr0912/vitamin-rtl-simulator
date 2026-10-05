module top;
  localparam logic [39:0] PA [2] = '{40'h10_0000_000C, 40'h0};
  if (PA[0] inside {4'b1?00}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
