module top;
  localparam int IA [2] = '{12, 3};
  if (IA[0] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
