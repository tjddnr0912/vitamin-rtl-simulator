module top;

  localparam R = ({4'b1x00} ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
