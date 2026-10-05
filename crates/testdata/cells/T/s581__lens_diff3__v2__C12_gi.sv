module top;
  localparam int N2 = 2;
  if ({N2+1{2'b10}} ==? 6'b10_1?10) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
