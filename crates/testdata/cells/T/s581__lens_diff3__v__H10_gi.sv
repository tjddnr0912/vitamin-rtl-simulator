module top;
  localparam logic signed [64:0] S65P = 65'sd12;
  if (S65P ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
