module top;
  function automatic logic [3:0] fv(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    fv = {3'b000, t[a - 2]};
  endfunction
  function automatic logic [3:0] fw(input int a);
    logic [3:0] t;
    unique if (a == 1) t = 4'd1;
    fw = {3'b000, ^t};
  endfunction
  localparam logic [3:0] V = fv(2);
  localparam logic [3:0] X = fw(2);
  initial begin #1 $display("V=%b X=%b", V, X); $finish; end
endmodule
