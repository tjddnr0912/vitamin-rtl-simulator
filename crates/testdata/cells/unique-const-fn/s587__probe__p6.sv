module top;
  function automatic int f(input int a);
    f = a + 5;
    $display("in f");
  endfunction
  function automatic int g(input int a);
    case (a) 2: g = 7; default: g = 3; endcase
  endfunction
  logic [31:0] r1, r2, v = '1;
  initial begin #1 r1 = {f(2){1'b1}}; r2 = {g(2){1'b1}}; $display("r1=%h r2=%h pw=%h", r1, r2, v[0 +: g(2)]); $finish; end
endmodule
