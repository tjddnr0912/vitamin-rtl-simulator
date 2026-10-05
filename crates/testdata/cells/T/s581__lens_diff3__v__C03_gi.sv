module top;
  localparam int N2 = 2;
  if ({N2{4'b1100}} ==? 'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
