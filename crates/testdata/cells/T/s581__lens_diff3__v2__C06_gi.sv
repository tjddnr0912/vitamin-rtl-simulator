module top;
  localparam int N2 = 2;
  if ({N2{2'b11}} ==? 4'b1?11) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
