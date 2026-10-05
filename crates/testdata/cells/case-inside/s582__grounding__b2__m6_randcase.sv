module top;
  int m;
  initial begin randcase 1: m = 1; 0: m = 2; endcase $display("m=%0d", m); #10 $finish; end
endmodule
