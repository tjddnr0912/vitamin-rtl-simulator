interface ifc;
  logic [1:0] y;
  task automatic t(input logic a, input logic b);
    unique if (a) y = 1; else if (b) y = 2;
  endtask
endinterface
program prg(input logic a, input logic b);
  logic [1:0] y;
  initial begin
    #3 unique if (a) y = 1; else if (b) y = 2;
    $display("t=%0t program done", $time);
  end
endprogram
module top;
  logic [1:0] sel; logic [1:0] y [2];
  ifc i();
  prg p(.a(1'b0), .b(1'b0));
  for (genvar g = 0; g < 2; g++) begin : gen
    always @(sel) begin
      unique if (sel[g]) y[g] = 1; else if (sel[1-g]) y[g] = 2;
    end
  end
  initial begin
    #1 sel = 2'b01;
    #1 sel = 2'b00;
    #2 i.t(1'b0, 1'b0); $display("t=%0t intf task done", $time);
    #1 sel = 2'b10;
    #1 $finish;
  end
  initial #100 $finish;
endmodule
