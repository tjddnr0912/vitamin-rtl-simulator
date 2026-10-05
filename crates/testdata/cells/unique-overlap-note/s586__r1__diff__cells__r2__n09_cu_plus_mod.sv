function automatic int f(int x);
  unique case (x) 0: return 1; default: return 2; endcase
endfunction
module top; initial $display("f=%0d", f(0)); endmodule
