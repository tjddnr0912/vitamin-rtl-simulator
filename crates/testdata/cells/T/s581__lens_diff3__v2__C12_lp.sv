module top;
  localparam int N2 = 2;
  localparam R = ({N2+1{2'b10}} ==? 6'b10_1?10);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
