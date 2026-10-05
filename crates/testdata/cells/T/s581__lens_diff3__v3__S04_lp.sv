module top;
  localparam shortint unsigned PSU = 16'hFFFC;
  localparam R = (PSU ==? 4'sb1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
