module top;
  localparam int N2 = 2;
  if ({N2{4'b1100}} == 8'b1100_1100) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
