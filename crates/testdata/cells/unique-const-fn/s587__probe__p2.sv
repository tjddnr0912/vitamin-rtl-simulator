module top;
  function automatic int g(input int a);
    g = 7;
    case (a) 1: g = 10; 2: g = 20; endcase
  endfunction
  function automatic int h(input int a);
    h = 7;
    unique case (a) 1: h = 10; 2: h = 20; endcase
  endfunction
  localparam int P = g(2);
  localparam int Q = h(2);
  localparam int R = h(3);
  initial begin #1 $display("P=%0d Q=%0d R=%0d", P, Q, R); $finish; end
endmodule
