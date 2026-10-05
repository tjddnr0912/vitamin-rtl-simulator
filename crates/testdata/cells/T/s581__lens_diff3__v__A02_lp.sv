module top;

  localparam R = ($unsigned(4'd12) ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
