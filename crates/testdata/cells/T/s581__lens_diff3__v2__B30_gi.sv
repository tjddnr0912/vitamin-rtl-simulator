module top;
  localparam logic [1:0][69:0] PP70 = {70'hC, 70'h0};
  if (PP70[1] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
