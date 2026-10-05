module top #(parameter type T = logic [39:0], parameter T P = 40'hC);

  localparam R = (P ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
