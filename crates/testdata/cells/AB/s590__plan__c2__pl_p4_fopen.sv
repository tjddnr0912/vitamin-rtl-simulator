module top;
  logic a; logic [31:0] y;
  function integer f(input logic x);
    integer fd;
    fd = $fopen("pl_p4_out.txt", "a");
    $fclose(fd);
    return fd;
  endfunction
  assign y = f(a);
  initial begin
    a = 0;
    #1 $display("t=%0t", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
