`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
`define SH(k, e) $display(`"k %b`", e)
package pk2;
  function automatic logic pfu(input logic [35:0] v); return `IN(v, 'bx1); endfunction
  function automatic logic pfs(input logic [35:0] v); return `IN(v, 32'bx1); endfunction
endpackage
module t;
  localparam C1 = 1'b1;
  localparam [35:0] V36 = 36'hF_0000_0001;
`ifndef IV
  let LU = 'bx1; let LU2 = LU; let LA(a) = a; let LS = 2'bx1;
 `define LETU LU
 `define LETU2 LU2
 `define LETA LA('bx1)
 `define LETS LS
`else
 `define LETU 'bx1
 `define LETU2 'bx1
 `define LETA 'bx1
 `define LETS 2'bx1
`endif
  localparam K01 = `IN(V36, 32'bx1);
  localparam K02 = `IN(V36, 'bx1);
  localparam K03 = `IN(V36, ('bx1));
`ifdef CMP
  localparam K04 = `IN(V36, (C1 ? 'bx1 : 'b0));
`endif
  logic [35:0] v36; logic [39:0] v40; logic c1; logic r1, r2;
  function automatic logic fu(input logic [35:0] v); return `IN(v, 'bx1); endfunction
  function automatic logic fs(input logic [35:0] v); return `IN(v, 32'bx1); endfunction
  task automatic tk(input logic [35:0] v, output logic r); r = `IN(v, 'bx1); endtask
  initial begin
    v36 = 36'hF_0000_0001; v40 = 40'hFF_0000_0001; c1 = 1;
    #1;
    `SH(U13, `IN(v36, 32'bx1));
    `SH(U01, `IN(v36, 'bx1));
    `SH(U12, `IN(v36, 2'bx1));
    `SH(U14, `IN(v36, 32'bx1));
    `SH(U02, `IN(v36, ('bx1)));
    `SH(U03, `IN(v36, `LETU));
    `SH(U04, `IN(v36, `LETU2));
    `SH(U05, `IN(v36, `LETA));
    `SH(U06, `IN(v36, `LETS));
`ifdef CMP
    `SH(U07, (v36 ==? (C1 ? 'bx1 : 'b0)));
`endif
`ifdef CMP
    `SH(U09, (v40 ==? $signed('bx1)));
`endif
`ifdef CMP
    `SH(U10, ($signed(v40) ==? $signed('bx1)));
`endif
`ifdef CMP
    `SH(U11, (v40 ==? 32'('bx1)));
`endif
    `SH(U15, `IN(v40, 'hx_0000_0001));
    `SH(U16, `IN(v40, 'h?));
    `SH(U17, `IN(v40, 'h0?));
    `SH(F01, fu(v36)); `SH(F02, fs(v36)); `SH(F03, pk2::pfu(v36)); `SH(F04, pk2::pfs(v36));
    tk(v36, r1); `SH(F05, r1);
`ifdef CMP
    `SH(K01, K01); `SH(K02, K02); `SH(K03, K03); `SH(K04, K04);
`endif
`ifdef RTT
    `SH(U08, (v36 ==? (c1 ? 'bx1 : 'b0)));
`endif
    #1 $finish;
  end
endmodule
