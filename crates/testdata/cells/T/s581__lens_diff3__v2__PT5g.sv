module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  if (PV ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
endmodule
module top;
  sub #(.T(logic signed [63:0]), .PV(-64'sd4)) u();
  initial #100 $finish;
endmodule
