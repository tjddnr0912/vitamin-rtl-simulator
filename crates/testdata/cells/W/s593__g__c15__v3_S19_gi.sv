module top;
  localparam bit signed [63:0] PBS = -64'sd4;
  if (PBS ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
