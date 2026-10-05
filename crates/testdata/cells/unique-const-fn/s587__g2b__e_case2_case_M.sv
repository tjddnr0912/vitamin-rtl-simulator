module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  int s = 7;
  initial begin #1 unique case (s) f(2): $display("ci=match"); 10: $display("ci=10"); endcase $finish; end
endmodule
