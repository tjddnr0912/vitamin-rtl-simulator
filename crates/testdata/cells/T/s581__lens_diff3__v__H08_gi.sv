module top;
  localparam logic signed [64:0] S65N = -65'sd4;
  if (S65N ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
