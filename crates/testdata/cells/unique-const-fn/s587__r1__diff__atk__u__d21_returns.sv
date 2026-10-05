module top;
  function automatic int fr1(input int a);
    if (a == 2) return 5;
    unique if (a == 1) return 10;
    return 7;
  endfunction
  function automatic int fr2(input int a);
    unique if (a == 1) return 10;
    return 7;
  endfunction
  function automatic int fr3(input int a);
    fr3 = 1;
    unique if (a == 1) fr3 = 10;
    else if (a == 3) return 3;
    fr3 = fr3 + 1;
  endfunction
  localparam int P1 = fr1(2);
  localparam int P2 = fr2(2);
  localparam int P3 = fr3(2);
  initial begin #1 $display("P1=%0d P2=%0d P3=%0d", P1, P2, P3); $finish; end
endmodule
