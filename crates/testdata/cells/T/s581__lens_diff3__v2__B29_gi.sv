module top;
  localparam logic [69:0] PA70 [2] = '{70'hC, 70'h0};
  if (PA70[0] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
